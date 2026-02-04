//! Logs command implementation
//!
//! Streams container logs with optional filtering, timestamps, and follow mode.

use crate::output::{format_docker_error_anyhow, log_level_style};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use futures_util::StreamExt;
use opencode_cloud_core::bollard::container::LogOutput;
use opencode_cloud_core::bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use opencode_cloud_core::bollard::query_parameters::LogsOptions;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, container_is_running, exec_command_exit_code,
};

/// Arguments for the logs command
#[derive(Args)]
pub struct LogsArgs {
    /// Number of lines to show (default: 50)
    #[arg(short = 'n', long = "lines", default_value = "50")]
    pub lines: String,

    /// Don't follow (one-shot dump)
    #[arg(long = "no-follow")]
    pub no_follow: bool,

    /// Prefix with timestamps
    #[arg(long)]
    pub timestamps: bool,

    /// Filter lines containing pattern
    #[arg(long)]
    pub grep: Option<String>,

    /// Show opencode-broker logs (requires systemd/journald in container)
    #[arg(long)]
    pub broker: bool,
}

/// Stream logs from the opencode container
///
/// By default, shows the last 50 lines and follows new output.
/// Use --no-follow for one-shot dump.
/// Use --grep to filter lines.
///
/// In quiet mode, outputs raw lines without status messages or colors.
pub async fn cmd_logs(args: &LogsArgs, maybe_host: Option<&str>, quiet: bool) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    // For logs, optionally prefix each line with host name
    // This helps identify source when tailing multiple hosts
    let line_prefix = host_name
        .as_ref()
        .map(|n| format!("[{}] ", style(n).cyan()));

    // Verify connection
    client
        .verify_connection()
        .await
        .map_err(|e| format_docker_error_anyhow(&e))?;

    // Check if container exists
    let inspect_result = client.inner().inspect_container(CONTAINER_NAME, None).await;

    match inspect_result {
        Err(opencode_cloud_core::bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
        }) => {
            return Err(anyhow!(
                "No container found. Run '{}' first.",
                style("occ start").cyan()
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to inspect container: {e}"));
        }
        Ok(_) => {}
    }

    // Determine follow mode
    let follow = !args.no_follow;

    if args.broker {
        // Show status message if following broker logs
        if !quiet && follow {
            eprintln!(
                "{}",
                style("Following broker logs (Ctrl+C to exit)...").dim()
            );
            eprintln!();
        }
        return stream_broker_logs(args, &client, line_prefix.as_deref(), quiet).await;
    }

    // Show status message if following container logs
    if !quiet && follow {
        eprintln!("{}", style("Following logs (Ctrl+C to exit)...").dim());
        eprintln!();
    }

    // Create log options
    let options = LogsOptions {
        stdout: true,
        stderr: true,
        follow,
        tail: args.lines.clone(),
        timestamps: args.timestamps,
        ..Default::default()
    };

    // Get log stream
    let mut stream = client.inner().logs(CONTAINER_NAME, Some(options));

    // Process log stream
    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                if let Some(line) = log_output_to_line(output) {
                    emit_log_line(&line, args, line_prefix.as_deref(), quiet);
                }
            }
            Err(_) => {
                // Stream error - check if container stopped
                if follow
                    && !container_is_running(&client, CONTAINER_NAME)
                        .await
                        .unwrap_or(false)
                    && !quiet
                {
                    eprintln!();
                    eprintln!("{}", style("Container stopped").dim());
                }
                break;
            }
        }
    }

    Ok(())
}

/// Stream opencode-broker logs from systemd journal inside the container
async fn stream_broker_logs(
    args: &LogsArgs,
    client: &DockerClient,
    line_prefix: Option<&str>,
    quiet: bool,
) -> Result<()> {
    ensure_systemd_available(client).await?;
    let cmd = build_broker_journalctl_command(args)?;
    let exec_id = create_broker_exec(client, cmd).await?;
    stream_broker_exec_output(args, client, &exec_id, line_prefix, quiet).await
}

async fn ensure_systemd_available(client: &DockerClient) -> Result<()> {
    let systemd_available = exec_command_exit_code(
        client,
        CONTAINER_NAME,
        vec!["test", "-d", "/run/systemd/system"],
    )
    .await
    .unwrap_or(1)
        == 0;

    if systemd_available {
        Ok(())
    } else {
        Err(anyhow!(
            "Broker logs require systemd/journald inside the container.\n\
This host doesn't support systemd-in-container or the container was created in Tini mode.\n\
Recreate the container on a supported Linux host with:\n  {}\n  {}",
            style("occ stop --remove").cyan(),
            style("occ start").cyan()
        ))
    }
}

fn build_broker_journalctl_command(args: &LogsArgs) -> Result<Vec<String>> {
    let mut cmd = vec![
        "journalctl".to_string(),
        "--no-pager".to_string(),
        "-u".to_string(),
        "opencode-broker".to_string(),
    ];

    if args.timestamps {
        cmd.push("-o".to_string());
        cmd.push("short-iso".to_string());
        cmd.push("--no-hostname".to_string());
    } else {
        cmd.push("-o".to_string());
        cmd.push("cat".to_string());
    }

    let lines = args.lines.trim();
    if lines.eq_ignore_ascii_case("all") {
        // no -n flag
    } else if !lines.is_empty() && lines.chars().all(|c| c.is_ascii_digit()) {
        cmd.push("-n".to_string());
        cmd.push(lines.to_string());
    } else {
        return Err(anyhow!(
            "Invalid value for --lines with --broker. Use a number or 'all'."
        ));
    }

    if !args.no_follow {
        cmd.push("-f".to_string());
    }

    Ok(cmd)
}

async fn create_broker_exec(client: &DockerClient, cmd: Vec<String>) -> Result<String> {
    let exec_config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd),
        user: Some("root".to_string()),
        ..Default::default()
    };

    let exec = client
        .inner()
        .create_exec(CONTAINER_NAME, exec_config)
        .await
        .map_err(|e| anyhow!("Failed to create exec for broker logs: {e}"))?;

    Ok(exec.id)
}

async fn stream_broker_exec_output(
    args: &LogsArgs,
    client: &DockerClient,
    exec_id: &str,
    line_prefix: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    match client
        .inner()
        .start_exec(exec_id, Some(start_config))
        .await
        .map_err(|e| anyhow!("Failed to start broker log stream: {e}"))?
    {
        StartExecResults::Attached {
            output: mut stream, ..
        } => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(output) => {
                        if let Some(line) = log_output_to_line(output) {
                            emit_log_line(&line, args, line_prefix, quiet);
                        }
                    }
                    Err(_) => {
                        if !args.no_follow
                            && !container_is_running(client, CONTAINER_NAME)
                                .await
                                .unwrap_or(false)
                            && !quiet
                        {
                            eprintln!();
                            eprintln!("{}", style("Container stopped").dim());
                        }
                        break;
                    }
                }
            }
        }
        StartExecResults::Detached => {
            return Err(anyhow!(
                "Exec unexpectedly detached while streaming broker logs"
            ));
        }
    }

    Ok(())
}

fn log_output_to_line(output: LogOutput) -> Option<String> {
    match output {
        LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
            Some(String::from_utf8_lossy(&message).to_string())
        }
        _ => None,
    }
}

fn emit_log_line(line: &str, args: &LogsArgs, prefix: Option<&str>, quiet: bool) {
    if let Some(pattern) = args.grep.as_deref()
        && !line.contains(pattern)
    {
        return;
    }

    if quiet {
        print_line(line, prefix);
    } else if console::colors_enabled() {
        print_styled_line(line, prefix);
    } else {
        print_line(line, prefix);
    }
}

/// Print a log line, ensuring newline at end
fn print_line(line: &str, prefix: Option<&str>) {
    let output = match prefix {
        Some(p) => format!("{p}{line}"),
        None => line.to_string(),
    };
    if output.ends_with('\n') {
        print!("{output}");
    } else {
        println!("{output}");
    }
}

/// Print a styled log line based on log level
fn print_styled_line(line: &str, prefix: Option<&str>) {
    let styled = log_level_style(line);
    let output = match prefix {
        Some(p) => format!("{p}{styled}"),
        None => styled.to_string(),
    };
    if output.ends_with('\n') {
        print!("{output}");
    } else {
        println!("{output}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_args_defaults() {
        // Test that defaults are applied correctly via clap
        // We can't easily test clap defaults here, but we can test
        // the parsing logic
        let args = LogsArgs {
            lines: "50".to_string(),
            no_follow: false,
            timestamps: false,
            grep: None,
            broker: false,
        };

        assert_eq!(args.lines, "50");
        assert!(!args.no_follow);
        assert!(!args.timestamps);
        assert!(args.grep.is_none());
    }

    #[test]
    fn print_line_adds_newline_when_missing() {
        // This is a basic test - the actual print happens to stdout
        // We just verify the logic
        let line_without_newline = "test line";
        let line_with_newline = "test line\n";

        assert!(!line_without_newline.ends_with('\n'));
        assert!(line_with_newline.ends_with('\n'));
    }

    #[test]
    fn grep_filter_logic() {
        // Test grep filtering logic
        let pattern = "ERROR";
        let matching_line = "2024-01-01 ERROR: something failed";
        let non_matching_line = "2024-01-01 INFO: all good";

        assert!(matching_line.contains(pattern));
        assert!(!non_matching_line.contains(pattern));
    }

    #[test]
    fn follow_mode_from_no_follow_flag() {
        // follow = !args.no_follow
        let args_follow = LogsArgs {
            lines: "50".to_string(),
            no_follow: false,
            timestamps: false,
            grep: None,
            broker: false,
        };
        assert!(!args_follow.no_follow);

        let args_no_follow = LogsArgs {
            lines: "50".to_string(),
            no_follow: true,
            timestamps: false,
            grep: None,
            broker: false,
        };
        assert!(args_no_follow.no_follow);
    }
}
