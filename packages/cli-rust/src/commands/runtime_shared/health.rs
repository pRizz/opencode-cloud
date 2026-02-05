//! Shared health mapping logic for host/container runtime parity.

use super::status_model::{BrokerHealthStatus, OpencodeHealthStatus, OpencodeHttpProbe};

pub fn map_broker_health_status(process_ok: bool, socket_ok: bool) -> BrokerHealthStatus {
    match (process_ok, socket_ok) {
        (true, true) => BrokerHealthStatus::Healthy,
        (true, false) | (false, true) => BrokerHealthStatus::Degraded,
        (false, false) => BrokerHealthStatus::Unhealthy,
    }
}

pub fn map_opencode_health_status(probe: OpencodeHttpProbe) -> OpencodeHealthStatus {
    match probe {
        OpencodeHttpProbe::Healthy => OpencodeHealthStatus::Healthy,
        OpencodeHttpProbe::ConnectionRefused | OpencodeHttpProbe::Timeout => {
            OpencodeHealthStatus::Starting
        }
        OpencodeHttpProbe::Unhealthy(code) => OpencodeHealthStatus::Unhealthy(code),
        OpencodeHttpProbe::Failed => OpencodeHealthStatus::CheckFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_health_mapping_healthy() {
        assert_eq!(
            map_broker_health_status(true, true),
            BrokerHealthStatus::Healthy
        );
    }

    #[test]
    fn broker_health_mapping_degraded_process_only() {
        assert_eq!(
            map_broker_health_status(true, false),
            BrokerHealthStatus::Degraded
        );
    }

    #[test]
    fn broker_health_mapping_degraded_socket_only() {
        assert_eq!(
            map_broker_health_status(false, true),
            BrokerHealthStatus::Degraded
        );
    }

    #[test]
    fn broker_health_mapping_unhealthy() {
        assert_eq!(
            map_broker_health_status(false, false),
            BrokerHealthStatus::Unhealthy
        );
    }

    #[test]
    fn opencode_health_mapping_healthy() {
        assert_eq!(
            map_opencode_health_status(OpencodeHttpProbe::Healthy),
            OpencodeHealthStatus::Healthy
        );
    }

    #[test]
    fn opencode_health_mapping_connection_refused_is_starting() {
        assert_eq!(
            map_opencode_health_status(OpencodeHttpProbe::ConnectionRefused),
            OpencodeHealthStatus::Starting
        );
    }

    #[test]
    fn opencode_health_mapping_timeout_is_starting() {
        assert_eq!(
            map_opencode_health_status(OpencodeHttpProbe::Timeout),
            OpencodeHealthStatus::Starting
        );
    }

    #[test]
    fn opencode_health_mapping_unhealthy_preserves_code() {
        assert_eq!(
            map_opencode_health_status(OpencodeHttpProbe::Unhealthy(503)),
            OpencodeHealthStatus::Unhealthy(503)
        );
    }

    #[test]
    fn opencode_health_mapping_failed() {
        assert_eq!(
            map_opencode_health_status(OpencodeHttpProbe::Failed),
            OpencodeHealthStatus::CheckFailed
        );
    }
}
