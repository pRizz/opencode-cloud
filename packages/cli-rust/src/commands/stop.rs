//! Stop command implementation
//!
//! Stops the opencode service with a graceful timeout.
//! Docker sends SIGTERM first, then SIGKILL if timeout expires.

use crate::commands::service::{StopSpinnerMessages, stop_service_with_spinner};
use crate::output::format_docker_error;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DEFAULT_STOP_TIMEOUT_SECS, container_is_running,
};

/// Arguments for the stop command
#[derive(Args, Default)]
pub struct StopArgs {
    /// Graceful shutdown timeout in seconds (default: 30)
    #[arg(long, short, default_value_t = DEFAULT_STOP_TIMEOUT_SECS)]
    pub timeout: i64,
}

/// Stop the opencode service
///
/// This command:
/// 1. Connects to Docker
/// 2. Checks if service is running (idempotent - exits 0 if already stopped)
/// 3. Stops the container with graceful timeout (default 30s)
pub async fn cmd_stop(args: &StopArgs, maybe_host: Option<&str>, quiet: bool) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    // Verify connection
    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{msg}")
    })?;

    // Check if already stopped (idempotent behavior)
    if !container_is_running(&client, CONTAINER_NAME).await? {
        if !quiet {
            let msg =
                crate::format_host_message(host_name.as_deref(), "Service is already stopped");
            println!("{}", style(msg).dim());
        }
        return Ok(());
    }

    stop_service_with_spinner(
        &client,
        host_name.as_deref(),
        quiet,
        false,
        args.timeout,
        true,
        StopSpinnerMessages {
            action_message: "Stopping service...",
            update_label: "Stopping service",
            success_base_message: "Service stopped",
            failure_message: "Failed to stop",
        },
    )
    .await?;

    Ok(())
}
