//! Centralized Docker error formatting
//!
//! This module provides consistent, actionable error messages for Docker-related
//! errors across all CLI commands.

use anyhow::anyhow;
use console::style;
use opencode_cloud_core::docker::DockerError;

/// Format Docker errors with actionable guidance
///
/// Returns a styled, multi-line error message with troubleshooting steps.
/// This is the most complete version with documentation links.
pub fn format_docker_error(e: &DockerError) -> String {
    match e {
        DockerError::NotRunning => {
            format!(
                "{}\n\n  {}\n  {}\n  {}\n\n  {}: {}",
                style("Docker is not responding").red().bold(),
                "Start or restart the Docker daemon:",
                style("  Linux:  sudo systemctl start docker").cyan(),
                style("  Linux:  sudo systemctl restart docker").cyan(),
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::SocketNotFound => {
            format!(
                "{}\n\n  {}\n  {}\n  {}\n  {}\n\n  {}: {}",
                style("Docker socket not found").red().bold(),
                "Docker may not be installed or the service isn't running:",
                style("  Linux:  sudo apt-get install docker.io").cyan(),
                style("  Linux:  sudo systemctl enable --now docker").cyan(),
                "Then verify the socket exists at /var/run/docker.sock (Linux default).",
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::PermissionDenied => {
            format!(
                "{}\n\n  {}\n  {}\n  {}\n\n  {}\n  {}\n  {}\n\n  {}: {}",
                style("Permission denied accessing Docker").red().bold(),
                "Your user likely lacks access to the Docker socket.",
                style("  Check: docker ps").cyan(),
                style("  Check: ls -l /var/run/docker.sock").cyan(),
                "Fix (Linux):",
                style("  sudo usermod -aG docker $USER").cyan(),
                "Then log out and back in (or run: newgrp docker).",
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::Connection(msg) => {
            format!(
                "{}\n\n  {}\n\n  {}: {}",
                style("Cannot connect to Docker").red().bold(),
                msg,
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::Container(msg) if msg.contains("port") => {
            format!(
                "{}\n\n  {}\n  {}\n\n  {}: {}",
                style("Port conflict").red().bold(),
                msg,
                style("  Try: occ start --port <different-port>").cyan(),
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        _ => e.to_string(),
    }
}

/// Format Docker errors as anyhow::Error
///
/// Convenience wrapper for commands that want to return the error directly.
pub fn format_docker_error_anyhow(e: &DockerError) -> anyhow::Error {
    anyhow!("{}", format_docker_error(e))
}

/// Show Docker error in a rich format to stderr
///
/// Prints a blank line before the error message for visual separation.
pub fn show_docker_error(e: &DockerError) {
    let msg = format_docker_error(e);
    eprintln!();
    eprintln!("{msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_docker_error_not_running() {
        let error = DockerError::NotRunning;
        let msg = format_docker_error(&error);
        assert!(msg.contains("Docker is not responding"));
        assert!(msg.contains("systemctl start docker"));
    }

    #[test]
    fn format_docker_error_socket_not_found() {
        let error = DockerError::SocketNotFound;
        let msg = format_docker_error(&error);
        assert!(msg.contains("Docker socket not found"));
        assert!(msg.contains("/var/run/docker.sock"));
    }

    #[test]
    fn format_docker_error_permission_denied() {
        let error = DockerError::PermissionDenied;
        let msg = format_docker_error(&error);
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("usermod"));
    }

    #[test]
    fn format_docker_error_connection() {
        let error = DockerError::Connection("socket not found".to_string());
        let msg = format_docker_error(&error);
        assert!(msg.contains("Cannot connect to Docker"));
        assert!(msg.contains("socket not found"));
    }

    #[test]
    fn format_docker_error_port_conflict() {
        let error = DockerError::Container("port 3000 already in use".to_string());
        let msg = format_docker_error(&error);
        assert!(msg.contains("Port conflict"));
        assert!(msg.contains("--port"));
    }

    #[test]
    fn format_docker_error_anyhow_wraps_correctly() {
        let error = DockerError::NotRunning;
        let anyhow_err = format_docker_error_anyhow(&error);
        let err_msg = anyhow_err.to_string();
        assert!(err_msg.contains("Docker is not responding"));
    }
}
