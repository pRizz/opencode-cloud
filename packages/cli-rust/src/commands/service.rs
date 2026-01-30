//! Shared helpers for service lifecycle commands
//!
//! Provides common stop behavior with spinner output.

use crate::output::{CommandSpinner, show_docker_error};
use anyhow::Result;
use console::style;
use opencode_cloud_core::docker::{DockerClient, stop_service};
use std::io::IsTerminal;
use std::time::Instant;
use tokio::io::AsyncBufReadExt;

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
    let is_tty = std::io::stdin().is_terminal();
    let enter_hint = if is_tty {
        " Press Enter to force kill now."
    } else {
        ""
    };
    spinner.update(&crate::format_host_message(
        host_name,
        &format!(
            "{} ({}s graceful timeout, then force kill)...{}",
            messages.update_label, timeout_secs, enter_hint
        ),
    ));

    let start = Instant::now();
    if !is_tty {
        return handle_stop_result(
            stop_service(client, remove, Some(timeout_secs)).await,
            start,
            timeout_secs,
            host_name,
            spinner,
            &messages,
        );
    }

    let stop_future = stop_service(client, remove, Some(timeout_secs));
    tokio::pin!(stop_future);
    let outcome = tokio::select! {
        result = &mut stop_future => StopOutcome::Graceful(result),
        _ = wait_for_enter() => {
            spinner.update(&crate::format_host_message(
                host_name,
                &format!("{} (forcing stop now)...", messages.update_label),
            ));
            StopOutcome::Forced(stop_service(client, remove, Some(0)).await)
        }
    };

    match outcome {
        StopOutcome::Graceful(result) => {
            handle_stop_result(result, start, timeout_secs, host_name, spinner, &messages)
        }
        StopOutcome::Forced(result) => handle_forced_result(result, host_name, spinner, &messages),
    }
}

async fn wait_for_enter() {
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input = String::new();
    let _ = stdin.read_line(&mut input).await;
}

fn handle_stop_result(
    result: Result<(), opencode_cloud_core::docker::DockerError>,
    start: Instant,
    timeout_secs: i64,
    host_name: Option<&str>,
    spinner: CommandSpinner,
    messages: &StopSpinnerMessages<'_>,
) -> Result<()> {
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

fn handle_forced_result(
    result: Result<(), opencode_cloud_core::docker::DockerError>,
    host_name: Option<&str>,
    spinner: CommandSpinner,
    messages: &StopSpinnerMessages<'_>,
) -> Result<()> {
    match result {
        Ok(()) => {
            let message = stop_success_message_forced(messages.success_base_message);
            spinner.success(&crate::format_host_message(host_name, &message));
            Ok(())
        }
        Err(error) => {
            spinner.fail(&crate::format_host_message(
                host_name,
                messages.failure_message,
            ));
            show_docker_error(&error);
            Err(error.into())
        }
    }
}

fn stop_success_message_forced(success_base_message: &str) -> String {
    format!("{success_base_message} (force killed on request)")
}

enum StopOutcome {
    Graceful(Result<(), opencode_cloud_core::docker::DockerError>),
    Forced(Result<(), opencode_cloud_core::docker::DockerError>),
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
