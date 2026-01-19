//! Stop command implementation
//!
//! Stops the opencode service with a graceful 30-second timeout.

use crate::output::CommandSpinner;
use anyhow::{anyhow, Result};
use clap::Args;
use console::style;
use opencode_cloud_core::docker::{
    container_is_running, stop_service, DockerClient, DockerError, CONTAINER_NAME,
};

/// Arguments for the stop command
#[derive(Args)]
pub struct StopArgs {
    // Future: --remove flag to also remove container
}

/// Stop the opencode service
///
/// This command:
/// 1. Connects to Docker
/// 2. Checks if service is running (idempotent - exits 0 if already stopped)
/// 3. Stops the container with 30s graceful timeout
pub async fn cmd_stop(_args: &StopArgs, quiet: bool) -> Result<()> {
    // Connect to Docker
    let client = connect_docker().await?;

    // Check if already stopped (idempotent behavior)
    if !container_is_running(&client, CONTAINER_NAME).await? {
        if !quiet {
            println!("{}", style("Service is already stopped").dim());
        }
        return Ok(());
    }

    // Create spinner
    let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
    spinner.update("Stopping service (30s timeout)...");

    // Stop with 30 second graceful timeout (passed via stop_container)
    match stop_service(&client, false).await {
        Ok(()) => {
            spinner.success("Service stopped");
        }
        Err(e) => {
            spinner.fail("Failed to stop");
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Connect to Docker with actionable error messages
async fn connect_docker() -> Result<DockerClient> {
    DockerClient::connect().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{}", msg)
    })
}

/// Format Docker errors with actionable guidance
fn format_docker_error(e: &DockerError) -> String {
    match e {
        DockerError::NotRunning => {
            format!(
                "{}\n\n  {}\n  {}",
                style("Docker is not running").red().bold(),
                "Start Docker Desktop or the Docker daemon:",
                style("  sudo systemctl start docker").cyan()
            )
        }
        DockerError::PermissionDenied => {
            format!(
                "{}\n\n  {}\n  {}\n  {}",
                style("Permission denied accessing Docker").red().bold(),
                "Add your user to the docker group:",
                style("  sudo usermod -aG docker $USER").cyan(),
                "Then log out and back in."
            )
        }
        _ => e.to_string(),
    }
}

/// Show Docker error in a rich format
fn show_docker_error(e: &DockerError) {
    let msg = format_docker_error(e);
    eprintln!();
    eprintln!("{}", msg);
}
