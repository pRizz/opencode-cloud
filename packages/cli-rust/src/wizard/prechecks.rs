//! Wizard prechecks
//!
//! Validates environment before running the setup wizard.

use anyhow::{Result, bail};
use opencode_cloud_core::docker::DockerClient;
use std::io::IsTerminal;

/// Verify Docker is available and running
///
/// Attempts to connect to Docker and verify the connection.
/// Returns actionable error if Docker is not available.
pub async fn verify_docker_available() -> Result<()> {
    let client = match DockerClient::new() {
        Ok(c) => c,
        Err(_) => {
            bail!(
                "Docker is not available.\n\n\
                Please start Docker Desktop or the Docker daemon before running setup.\n\n\
                On Linux:   sudo systemctl start docker\n\
                On macOS:   Open Docker Desktop application"
            );
        }
    };

    if client.verify_connection().await.is_err() {
        bail!(
            "Docker is not running.\n\n\
            Please start Docker Desktop or the Docker daemon before running setup.\n\n\
            On Linux:   sudo systemctl start docker\n\
            On macOS:   Open Docker Desktop application"
        );
    }

    Ok(())
}

/// Verify TTY is available for interactive prompts
///
/// Returns error with guidance if stdin is not a terminal.
pub fn verify_tty() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "No TTY detected. Use occ config set or provide config file.\n\n\
            For non-interactive setup, pre-set auth credentials then run commands:\n  \
            occ config set username <user>\n  \
            occ config set password  # will prompt\n\n\
            Or edit the config file directly:\n  \
            ~/.config/opencode-cloud/config.json"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests are intentionally minimal as they depend on environment state
    // (Docker running, TTY availability) that varies between CI and local development.

    #[test]
    fn test_verify_tty_runs() {
        // Just verify the function compiles and returns a Result
        // Actual TTY check depends on test environment
        let _ = verify_tty();
    }
}
