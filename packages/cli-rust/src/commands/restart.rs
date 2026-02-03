//! Restart command implementation
//!
//! Restarts the opencode service (stop + start).

use crate::commands::start::{wait_for_broker_ready, wait_for_service_ready};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::{CommandSpinner, format_docker_error, show_docker_error};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, container_is_running, setup_and_start, stop_service,
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
pub async fn cmd_restart(
    _args: &RestartArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    if verbose > 0 {
        let target = host_name.as_deref().unwrap_or("local");
        eprintln!(
            "{} Connecting to Docker on {}...",
            style("[info]").cyan(),
            target
        );
    }

    // Verify connection
    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{msg}")
    })?;

    // Load config for port and bind_address
    let config = load_config_or_default()?;
    let port = config.opencode_web_port;
    let bind_addr = &config.bind_address;

    // Create single spinner for the full operation
    let msg = crate::format_host_message(host_name.as_deref(), "Restarting service...");
    let spinner = CommandSpinner::new_maybe(&msg, quiet);

    // Stop if running
    if container_is_running(&client, CONTAINER_NAME).await? {
        spinner.update(&crate::format_host_message(
            host_name.as_deref(),
            "Stopping service...",
        ));
        if let Err(e) = stop_service(&client, false, None).await {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to stop",
            ));
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    // Start
    spinner.update(&crate::format_host_message(
        host_name.as_deref(),
        "Starting service...",
    ));
    match setup_and_start(
        &client,
        Some(port),
        None,
        Some(bind_addr),
        Some(config.cockpit_port),
        Some(config.cockpit_enabled && COCKPIT_EXPOSED),
        None, // bind_mounts: restart preserves existing container mounts
    )
    .await
    {
        Ok(container_id) => {
            if let Err(e) = wait_for_service_ready(&client, bind_addr, port, &spinner).await {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Service failed to become ready",
                ));
                return Err(e);
            }

            if let Err(e) = wait_for_broker_ready(&client, &spinner).await {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Broker failed to become ready",
                ));
                return Err(e);
            }

            spinner.success(&crate::format_host_message(
                host_name.as_deref(),
                "Service restarted",
            ));

            if !quiet {
                let url = format!("http://{bind_addr}:{port}");
                println!();
                println!("URL:        {}", style(&url).cyan());
                println!(
                    "Container:  {}",
                    style(&container_id[..12.min(container_id.len())]).dim()
                );
            }
        }
        Err(e) => {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to start",
            ));
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    Ok(())
}
