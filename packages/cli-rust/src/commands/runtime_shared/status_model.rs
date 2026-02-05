//! Shared runtime status model and formatting helpers.

use console::style;

/// Runtime constraints/capabilities used to normalize behavior across runtimes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeCapabilities {
    /// Whether systemd is available in this runtime.
    pub systemd_available: Option<bool>,
    /// Whether journalctl is available in this runtime.
    pub journalctl_available: Option<bool>,
    /// Whether user management operations require root privileges.
    pub root_required_for_user_management: Option<bool>,
}

/// Raw opencode HTTP probe result from a runtime backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpencodeHttpProbe {
    Healthy,
    ConnectionRefused,
    Timeout,
    Unhealthy(u16),
    Failed,
}

/// Normalized opencode health state used by host/container status outputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpencodeHealthStatus {
    Healthy,
    Starting,
    Unhealthy(u16),
    CheckFailed,
}

/// Normalized broker health state used by host/container status outputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrokerHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    CheckFailed,
}

/// Shared status snapshot collected through a runtime backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusViewModel {
    pub opencode_health: Option<OpencodeHealthStatus>,
    pub broker_health: BrokerHealthStatus,
    pub opencode_version: String,
    pub opencode_commit: String,
    pub image_version: String,
    pub capabilities: RuntimeCapabilities,
}

pub fn format_opencode_health_label(status: OpencodeHealthStatus) -> String {
    match status {
        OpencodeHealthStatus::Healthy => style("Healthy").green().to_string(),
        OpencodeHealthStatus::Starting => style("Service starting...").yellow().to_string(),
        OpencodeHealthStatus::Unhealthy(code) => {
            format!("{} (HTTP {})", style("Unhealthy").red(), code)
        }
        OpencodeHealthStatus::CheckFailed => style("Check failed").yellow().to_string(),
    }
}

pub fn format_broker_health_label(status: BrokerHealthStatus) -> String {
    match status {
        BrokerHealthStatus::Healthy => style("Healthy").green().to_string(),
        BrokerHealthStatus::Degraded => style("Degraded").yellow().to_string(),
        BrokerHealthStatus::Unhealthy => style("Unhealthy").red().to_string(),
        BrokerHealthStatus::CheckFailed => style("Check failed").yellow().to_string(),
    }
}
