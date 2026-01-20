//! Restart command implementation
//!
//! Restarts the opencode service (stop + start).

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::config::load_config;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, DockerError, container_is_running, setup_and_start, stop_service,
};

/// Arguments for the restart command
#[derive(Args)]
pub struct RestartArgs {
    // Future: --port flag to change port on restart
}

/// Restart the opencode service
///
/// This command:
/// 1. Connects to Docker
/// 2. Stops the service if running
/// 3. Starts the service
pub async fn cmd_restart(_args: &RestartArgs, quiet: bool, verbose: u8) -> Result<()> {
    // Connect to Docker
    let client = connect_docker(verbose)?;

    // Verify connection
    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{}", msg)
    })?;

    // Load config for port and bind_address
    let config = load_config()?;
    let port = config.opencode_web_port;
    let bind_addr = &config.bind_address;

    // Create single spinner for the full operation
    let spinner = CommandSpinner::new_maybe("Restarting service...", quiet);

    // Stop if running
    if container_is_running(&client, CONTAINER_NAME).await? {
        spinner.update("Stopping service...");
        if let Err(e) = stop_service(&client, false).await {
            spinner.fail("Failed to stop");
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    // Start
    spinner.update("Starting service...");
    match setup_and_start(&client, Some(port), None, Some(bind_addr)).await {
        Ok(container_id) => {
            spinner.success("Service restarted");

            if !quiet {
                let url = format!("http://{}:{}", bind_addr, port);
                println!();
                println!("URL:        {}", style(&url).cyan());
                println!(
                    "Container:  {}",
                    style(&container_id[..12.min(container_id.len())]).dim()
                );
            }
        }
        Err(e) => {
            spinner.fail("Failed to start");
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Connect to Docker with actionable error messages
fn connect_docker(verbose: u8) -> Result<DockerClient> {
    if verbose > 0 {
        eprintln!("{} Connecting to Docker...", style("[info]").cyan());
    }

    DockerClient::new().map_err(|e| {
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
