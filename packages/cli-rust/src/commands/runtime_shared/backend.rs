//! Runtime backends for shared host/container command behavior.

use crate::commands::container::{
    exec_command as exec_local_command, exec_command_with_status as exec_local_with_status,
};
use crate::output::normalize_bind_addr;
use anyhow::Result;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, HealthError, OPENCODE_WEB_PORT, check_health, exec_command,
    exec_command_with_status,
};
use std::path::Path;

use super::status_model::{OpencodeHttpProbe, RuntimeCapabilities};

pub trait RuntimeBackend {
    async fn probe_opencode_http_health(
        &self,
        bind_addr: &str,
        host_port: u16,
    ) -> Result<OpencodeHttpProbe>;
    async fn probe_broker_process_active(&self) -> Result<bool>;
    async fn probe_broker_socket_present(&self) -> Result<bool>;
    async fn read_opencode_version(&self) -> Result<Option<String>>;
    async fn read_opencode_commit(&self) -> Result<Option<String>>;
    async fn read_image_version(&self) -> Result<Option<String>>;
    fn runtime_capabilities(&self) -> RuntimeCapabilities;
}

pub struct HostBackend<'a> {
    client: &'a DockerClient,
}

impl<'a> HostBackend<'a> {
    pub fn new(client: &'a DockerClient) -> Self {
        Self { client }
    }
}

impl RuntimeBackend for HostBackend<'_> {
    async fn probe_opencode_http_health(
        &self,
        bind_addr: &str,
        host_port: u16,
    ) -> Result<OpencodeHttpProbe> {
        let probe = match check_health(normalize_bind_addr(bind_addr), host_port).await {
            Ok(_) => OpencodeHttpProbe::Healthy,
            Err(HealthError::ConnectionRefused) => OpencodeHttpProbe::ConnectionRefused,
            Err(HealthError::Timeout) => OpencodeHttpProbe::Timeout,
            Err(HealthError::Unhealthy(code)) => OpencodeHttpProbe::Unhealthy(code),
            Err(_) => OpencodeHttpProbe::Failed,
        };
        Ok(probe)
    }

    async fn probe_broker_process_active(&self) -> Result<bool> {
        let (_output, status) = exec_command_with_status(
            self.client,
            CONTAINER_NAME,
            vec![
                "sh",
                "-lc",
                "if [ -d /run/systemd/system ]; then systemctl is-active --quiet opencode-broker.service; else pgrep -x opencode-broker >/dev/null; fi",
            ],
        )
        .await?;
        Ok(status == 0)
    }

    async fn probe_broker_socket_present(&self) -> Result<bool> {
        let (_output, status) = exec_command_with_status(
            self.client,
            CONTAINER_NAME,
            vec!["sh", "-lc", "test -S /run/opencode/auth.sock"],
        )
        .await?;
        Ok(status == 0)
    }

    async fn read_opencode_version(&self) -> Result<Option<String>> {
        let output = match exec_command(
            self.client,
            CONTAINER_NAME,
            vec!["/opt/opencode/bin/opencode", "--version"],
        )
        .await
        {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };

        let version = output.lines().next().map(str::trim).unwrap_or_default();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version.to_string()))
        }
    }

    async fn read_opencode_commit(&self) -> Result<Option<String>> {
        let output = match exec_command(
            self.client,
            CONTAINER_NAME,
            vec!["cat", "/opt/opencode/COMMIT"],
        )
        .await
        {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };
        Ok(extract_short_commit(&output))
    }

    async fn read_image_version(&self) -> Result<Option<String>> {
        let output = match exec_command(
            self.client,
            CONTAINER_NAME,
            vec!["cat", "/etc/opencode-cloud-version"],
        )
        .await
        {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };

        let version = output.lines().next().map(str::trim).unwrap_or_default();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version.to_string()))
        }
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            systemd_available: None,
            journalctl_available: None,
            root_required_for_user_management: None,
        }
    }
}

pub struct ContainerBackend {
    systemd_available: bool,
}

impl ContainerBackend {
    pub fn new(systemd_available: bool) -> Self {
        Self { systemd_available }
    }
}

impl RuntimeBackend for ContainerBackend {
    async fn probe_opencode_http_health(
        &self,
        _bind_addr: &str,
        host_port: u16,
    ) -> Result<OpencodeHttpProbe> {
        let probe = match check_health("127.0.0.1", host_port).await {
            Ok(_) => OpencodeHttpProbe::Healthy,
            Err(HealthError::ConnectionRefused) => OpencodeHttpProbe::ConnectionRefused,
            Err(HealthError::Timeout) => OpencodeHttpProbe::Timeout,
            Err(HealthError::Unhealthy(code)) => OpencodeHttpProbe::Unhealthy(code),
            Err(_) => OpencodeHttpProbe::Failed,
        };
        Ok(probe)
    }

    async fn probe_broker_process_active(&self) -> Result<bool> {
        if self.systemd_available {
            let (_output, status) =
                exec_local_with_status("systemctl", &["is-active", "opencode-broker.service"])
                    .await?;
            return Ok(status == 0);
        }

        let (_output, status) = exec_local_with_status("pgrep", &["-x", "opencode-broker"]).await?;
        Ok(status == 0)
    }

    async fn probe_broker_socket_present(&self) -> Result<bool> {
        Ok(Path::new("/run/opencode/auth.sock").exists())
    }

    async fn read_opencode_version(&self) -> Result<Option<String>> {
        let output = match exec_local_command("/opt/opencode/bin/opencode", &["--version"]).await {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };
        let version = output.lines().next().map(str::trim).unwrap_or_default();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version.to_string()))
        }
    }

    async fn read_opencode_commit(&self) -> Result<Option<String>> {
        let contents = match std::fs::read_to_string("/opt/opencode/COMMIT") {
            Ok(contents) => contents,
            Err(_) => return Ok(None),
        };
        Ok(extract_short_commit(&contents))
    }

    async fn read_image_version(&self) -> Result<Option<String>> {
        let contents = match std::fs::read_to_string("/etc/opencode-cloud-version") {
            Ok(contents) => contents,
            Err(_) => return Ok(None),
        };
        let version = contents.lines().next().map(str::trim).unwrap_or_default();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version.to_string()))
        }
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            systemd_available: Some(self.systemd_available),
            journalctl_available: Some(self.systemd_available),
            root_required_for_user_management: Some(true),
        }
    }
}

pub fn default_container_port() -> u16 {
    OPENCODE_WEB_PORT
}

fn extract_short_commit(version_output: &str) -> Option<String> {
    version_output
        .split(|ch: char| !ch.is_ascii_hexdigit())
        .find(|token| {
            token.len() >= 7
                && token.chars().all(|ch| ch.is_ascii_hexdigit())
                && token.chars().any(|ch| matches!(ch, 'a'..='f' | 'A'..='F'))
        })
        .map(|token| token.chars().take(7).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_short_commit_from_commit_file() {
        let output = "df9b40be451372e5473b22b33a68fb359267ca7e\n";
        assert_eq!(extract_short_commit(output).as_deref(), Some("df9b40b"));
    }

    #[test]
    fn extract_short_commit_ignores_numeric_versions() {
        let output = "0.0.0--202601311855";
        assert!(extract_short_commit(output).is_none());
    }
}
