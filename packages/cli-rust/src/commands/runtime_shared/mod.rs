//! Shared runtime command core to keep host/container implementations aligned.

pub mod backend;
pub mod health;
pub mod mounts;
pub mod status_model;

use anyhow::Result;
use backend::RuntimeBackend;
use health::{map_broker_health_status, map_opencode_health_status};
use status_model::{BrokerHealthStatus, StatusViewModel};

pub async fn probe_broker_health<B: RuntimeBackend>(backend: &B) -> BrokerHealthStatus {
    let process_probe = backend.probe_broker_process_active().await;
    let socket_probe = backend.probe_broker_socket_present().await;

    match (process_probe, socket_probe) {
        (Ok(process_ok), Ok(socket_ok)) => map_broker_health_status(process_ok, socket_ok),
        _ => BrokerHealthStatus::CheckFailed,
    }
}

pub async fn collect_status_view<B: RuntimeBackend>(
    backend: &B,
    include_opencode_probe: bool,
    bind_addr: &str,
    host_port: u16,
) -> Result<StatusViewModel> {
    let opencode_health = if include_opencode_probe {
        let probe = match backend
            .probe_opencode_http_health(bind_addr, host_port)
            .await
        {
            Ok(probe) => probe,
            Err(_) => status_model::OpencodeHttpProbe::Failed,
        };
        Some(map_opencode_health_status(probe))
    } else {
        None
    };

    let broker_health = probe_broker_health(backend).await;

    let opencode_version = backend
        .read_opencode_version()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".to_string());
    let opencode_commit = backend
        .read_opencode_commit()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".to_string());
    let image_version = backend
        .read_image_version()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".to_string());

    Ok(StatusViewModel {
        opencode_health,
        broker_health,
        opencode_version,
        opencode_commit,
        image_version,
        capabilities: backend.runtime_capabilities(),
    })
}

pub fn broker_is_ready(status: BrokerHealthStatus) -> bool {
    matches!(status, BrokerHealthStatus::Healthy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::runtime_shared::status_model::OpencodeHttpProbe;

    #[derive(Clone)]
    struct FakeBackend {
        opencode_probe: OpencodeHttpProbe,
        broker_process: bool,
        broker_socket: bool,
        fail_opencode_probe: bool,
        fail_broker_process: bool,
        fail_broker_socket: bool,
        opencode_version: Option<String>,
        opencode_commit: Option<String>,
        image_version: Option<String>,
        capabilities: status_model::RuntimeCapabilities,
    }

    impl RuntimeBackend for FakeBackend {
        async fn probe_opencode_http_health(
            &self,
            _bind_addr: &str,
            _host_port: u16,
        ) -> Result<OpencodeHttpProbe> {
            if self.fail_opencode_probe {
                return Err(anyhow::anyhow!("opencode probe failed"));
            }
            Ok(self.opencode_probe)
        }

        async fn probe_broker_process_active(&self) -> Result<bool> {
            if self.fail_broker_process {
                return Err(anyhow::anyhow!("broker process probe failed"));
            }
            Ok(self.broker_process)
        }

        async fn probe_broker_socket_present(&self) -> Result<bool> {
            if self.fail_broker_socket {
                return Err(anyhow::anyhow!("broker socket probe failed"));
            }
            Ok(self.broker_socket)
        }

        async fn read_opencode_version(&self) -> Result<Option<String>> {
            Ok(self.opencode_version.clone())
        }

        async fn read_opencode_commit(&self) -> Result<Option<String>> {
            Ok(self.opencode_commit.clone())
        }

        async fn read_image_version(&self) -> Result<Option<String>> {
            Ok(self.image_version.clone())
        }

        fn runtime_capabilities(&self) -> status_model::RuntimeCapabilities {
            self.capabilities
        }
    }

    #[tokio::test]
    async fn parity_same_signals_same_semantic_statuses() {
        let host_backend = FakeBackend {
            opencode_probe: OpencodeHttpProbe::Healthy,
            broker_process: true,
            broker_socket: false,
            fail_opencode_probe: false,
            fail_broker_process: false,
            fail_broker_socket: false,
            opencode_version: Some("v1".to_string()),
            opencode_commit: Some("abcdef0".to_string()),
            image_version: Some("v2".to_string()),
            capabilities: status_model::RuntimeCapabilities {
                systemd_available: None,
                journalctl_available: None,
                root_required_for_user_management: None,
            },
        };

        let container_backend = FakeBackend {
            capabilities: status_model::RuntimeCapabilities {
                systemd_available: Some(true),
                journalctl_available: Some(true),
                root_required_for_user_management: Some(true),
            },
            ..host_backend.clone()
        };

        let host_view = collect_status_view(&host_backend, true, "127.0.0.1", 3000)
            .await
            .expect("host view");
        let container_view = collect_status_view(&container_backend, true, "127.0.0.1", 3000)
            .await
            .expect("container view");

        assert_eq!(host_view.opencode_health, container_view.opencode_health);
        assert_eq!(host_view.broker_health, container_view.broker_health);
    }

    #[tokio::test]
    async fn broker_probe_failure_maps_to_check_failed() {
        let backend = FakeBackend {
            opencode_probe: OpencodeHttpProbe::Healthy,
            broker_process: true,
            broker_socket: true,
            fail_opencode_probe: false,
            fail_broker_process: true,
            fail_broker_socket: false,
            opencode_version: Some("v1".to_string()),
            opencode_commit: Some("abcdef0".to_string()),
            image_version: Some("v2".to_string()),
            capabilities: status_model::RuntimeCapabilities::default(),
        };

        let view = collect_status_view(&backend, true, "127.0.0.1", 3000)
            .await
            .expect("view");
        assert_eq!(view.broker_health, BrokerHealthStatus::CheckFailed);
    }

    #[test]
    fn broker_ready_only_when_healthy() {
        assert!(broker_is_ready(BrokerHealthStatus::Healthy));
        assert!(!broker_is_ready(BrokerHealthStatus::Degraded));
        assert!(!broker_is_ready(BrokerHealthStatus::Unhealthy));
        assert!(!broker_is_ready(BrokerHealthStatus::CheckFailed));
    }
}
