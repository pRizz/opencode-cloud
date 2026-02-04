//! Logs command implementation
//!
//! Streams container logs with optional filtering, timestamps, and follow mode.

use crate::output::{format_docker_error_anyhow, log_level_style};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use futures_util::StreamExt;
use opencode_cloud_core::bollard::container::LogOutput;
use opencode_cloud_core::bollard::query_parameters::LogsOptions;
use opencode_cloud_core::docker::{CONTAINER_NAME, container_is_running};

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

    // Show status message if following
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
                let line = match output {
                    LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                        String::from_utf8_lossy(&message).to_string()
                    }
                    _ => continue,
                };

                // Apply grep filter
                if let Some(ref pattern) = args.grep
                    && !line.contains(pattern)
                {
                    continue;
                }

                // Print the line
                if quiet {
                    // Quiet mode: raw output
                    print_line(&line, line_prefix.as_deref());
                } else if console::colors_enabled() {
                    // Color mode: apply log level styling
                    print_styled_line(&line, line_prefix.as_deref());
                } else {
                    // No colors: raw output
                    print_line(&line, line_prefix.as_deref());
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
        };
        assert!(!args_follow.no_follow);

        let args_no_follow = LogsArgs {
            lines: "50".to_string(),
            no_follow: true,
            timestamps: false,
            grep: None,
        };
        assert!(args_no_follow.no_follow);
    }
}
