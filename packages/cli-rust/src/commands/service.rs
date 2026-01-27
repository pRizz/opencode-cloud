//! Shared helpers for service lifecycle commands
//!
//! Provides common stop behavior with spinner output.

use crate::output::{CommandSpinner, show_docker_error};
use anyhow::Result;
use console::style;
use opencode_cloud_core::docker::{DockerClient, stop_service};
use std::time::Instant;

pub struct StopSpinnerMessages<'a> {
    pub action_message: &'a str,
    pub update_label: &'a str,
    pub success_base_message: &'a str,
    pub failure_message: &'a str,
}

pub async fn stop_service_with_spinner(
    client: &DockerClient,
    host_name: Option<&str>,
    quiet: bool,
    remove: bool,
    timeout_secs: i64,
    messages: StopSpinnerMessages<'_>,
) -> Result<()> {
    if quiet {
        stop_service(client, remove, Some(timeout_secs)).await?;
        return Ok(());
    }

    let spinner = CommandSpinner::new_maybe(
        &crate::format_host_message(host_name, messages.action_message),
        quiet,
    );
    spinner.update(&crate::format_host_message(
        host_name,
        &format!(
            "{} ({}s graceful timeout, then force kill)...",
            messages.update_label, timeout_secs
        ),
    ));

    let start = Instant::now();
    let result = stop_service(client, remove, Some(timeout_secs)).await;
    let Err(error) = result else {
        let elapsed_secs = start.elapsed().as_secs();
        let (message, should_warn) =
            stop_success_message(messages.success_base_message, timeout_secs, elapsed_secs);
        spinner.success(&crate::format_host_message(host_name, &message));
        if should_warn {
            eprintln!(
                "{}",
                style("Note: Container did not stop gracefully within timeout.").dim()
            );
        }
        return Ok(());
    };

    spinner.fail(&crate::format_host_message(
        host_name,
        messages.failure_message,
    ));
    show_docker_error(&error);
    Err(error.into())
}

fn stop_success_message(
    success_base_message: &str,
    timeout_secs: i64,
    elapsed_secs: u64,
) -> (String, bool) {
    let should_warn = elapsed_secs >= timeout_secs as u64;
    let message = if should_warn {
        format!("{success_base_message} (force killed after {timeout_secs}s timeout)")
    } else {
        format!("{success_base_message} ({elapsed_secs}s)")
    };

    (message, should_warn)
}
