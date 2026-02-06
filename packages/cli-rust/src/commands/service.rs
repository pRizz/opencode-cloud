//! Shared helpers for service lifecycle commands
//!
//! Provides common stop behavior with spinner output.

use crate::output::{CommandSpinner, show_docker_error};
use anyhow::Result;
use console::style;
use opencode_cloud_core::docker::{DockerClient, stop_service};
use std::io::IsTerminal;
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

    let stdin_rx = spawn_enter_listener();

    let outcome = tokio::select! {
        result = &mut stop_future => {
            // stdin_rx is dropped, unblocking the oneshot.
            // The std::thread reading stdin will exit once the process does;
            // it is NOT in tokio's blocking pool so it won't block runtime shutdown.
            StopOutcome::Graceful(result)
        }
        join_result = stdin_rx => {
            if user_pressed_enter(join_result) {
                spinner.update(&crate::format_host_message(
                    host_name,
                    &format!("{} (forcing stop now)...", messages.update_label),
                ));
                StopOutcome::Forced(stop_service(client, remove, Some(0)).await)
            } else {
                StopOutcome::Graceful(stop_future.await)
            }
        }
    };

    match outcome {
        StopOutcome::Graceful(result) => {
            handle_stop_result(result, start, timeout_secs, host_name, spinner, &messages)
        }
        StopOutcome::Forced(result) => handle_forced_result(result, host_name, spinner, &messages),
    }
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

/// Spawns a detached thread that waits for the user to press Enter and signals
/// back through a oneshot channel.
///
/// Uses a plain `std::thread` instead of `tokio::task::spawn_blocking` so the
/// thread is **not** part of tokio's blocking pool. This way the runtime can
/// shut down cleanly even if the thread is still blocked on `read_line` — the
/// OS will terminate the thread when the process exits.
fn spawn_enter_listener() -> tokio::sync::oneshot::Receiver<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        let mut line = String::new();
        let pressed = match stdin.lock().read_line(&mut line) {
            Ok(n) => n > 0,
            Err(_) => false,
        };
        let _ = tx.send(pressed);
    });

    rx
}

/// Returns true if the stdin listener indicates the user pressed Enter.
fn user_pressed_enter(result: Result<bool, tokio::sync::oneshot::error::RecvError>) -> bool {
    matches!(result, Ok(true))
}
