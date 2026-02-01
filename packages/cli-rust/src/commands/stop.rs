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
    CONTAINER_NAME, DEFAULT_STOP_TIMEOUT_SECS, container_exists, container_is_running,
};

/// Arguments for the stop command
#[derive(Args, Default)]
pub struct StopArgs {
    /// Graceful shutdown timeout in seconds (default: 30)
    #[arg(long, short, default_value_t = DEFAULT_STOP_TIMEOUT_SECS)]
    pub timeout: i64,

    /// Remove the container after stopping
    #[arg(long)]
    pub remove: bool,
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

    let is_running = container_is_running(&client, CONTAINER_NAME).await?;
    let exists = container_exists(&client, CONTAINER_NAME).await?;
    if !is_running {
        handle_not_running(exists, args.remove, quiet, host_name.as_deref())?;
    }

    stop_service_with_spinner(
        &client,
        host_name.as_deref(),
        quiet,
        args.remove,
        args.timeout,
        StopSpinnerMessages {
            action_message: "Stopping service...",
            update_label: "Stopping service",
            success_base_message: stop_success_message(args.remove),
            failure_message: "Failed to stop",
        },
    )
    .await?;

    Ok(())
}

fn handle_not_running(
    exists: bool,
    remove: bool,
    quiet: bool,
    maybe_host_name: Option<&str>,
) -> Result<()> {
    if !exists {
        print_dimmed_status(
            quiet,
            maybe_host_name,
            "Service container is already removed",
        );
        return Ok(());
    }

    let false = remove else {
        print_dimmed_status(quiet, maybe_host_name, "Service is already stopped");
        return Ok(());
    };

    Ok(())
}

fn print_dimmed_status(quiet: bool, maybe_host_name: Option<&str>, message: &str) {
    if quiet {
        return;
    }

    let msg = crate::format_host_message(maybe_host_name, message);
    println!("{}", style(msg).dim());
}

fn stop_success_message(remove: bool) -> &'static str {
    if remove {
        "Service stopped and removed"
    } else {
        "Service stopped"
    }
}
