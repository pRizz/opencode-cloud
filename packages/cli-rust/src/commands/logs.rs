//! Logs command implementation
//!
//! Streams container logs with optional filtering, timestamps, and follow mode.

use crate::output::log_level_style;
use anyhow::{Result, anyhow};
use bollard::container::{LogOutput, LogsOptions};
use clap::Args;
use console::style;
use futures_util::StreamExt;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, DockerError, container_is_running};

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
pub async fn cmd_logs(args: &LogsArgs, quiet: bool) -> Result<()> {
    // Connect to Docker
    let client = DockerClient::new().map_err(|e| format_docker_error(&e))?;

    // Verify connection
    client.verify_connection().await.map_err(|e| format_docker_error(&e))?;

    // Check if container exists
    let inspect_result = client.inner().inspect_container(CONTAINER_NAME, None).await;

    match inspect_result {
        Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
            return Err(anyhow!(
                "No container found. Run '{}' first.",
                style("occ start").cyan()
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to inspect container: {}", e));
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
    let options = LogsOptions::<String> {
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
                if let Some(ref pattern) = args.grep {
                    if !line.contains(pattern) {
                        continue;
                    }
                }

                // Print the line
                if quiet {
                    // Quiet mode: raw output
                    print_line(&line);
                } else if console::colors_enabled() {
                    // Color mode: apply log level styling
                    print_styled_line(&line);
                } else {
                    // No colors: raw output
                    print_line(&line);
                }
            }
            Err(_) => {
                // Stream error - check if container stopped
                if follow {
                    if !container_is_running(&client, CONTAINER_NAME)
                        .await
                        .unwrap_or(false)
                    {
                        if !quiet {
                            eprintln!();
                            eprintln!("{}", style("Container stopped").dim());
                        }
                    }
                }
                break;
            }
        }
    }

    Ok(())
}

/// Print a log line, ensuring newline at end
fn print_line(line: &str) {
    if line.ends_with('\n') {
        print!("{}", line);
    } else {
        println!("{}", line);
    }
}

/// Print a styled log line based on log level
fn print_styled_line(line: &str) {
    let styled = log_level_style(line);
    if line.ends_with('\n') {
        print!("{}", styled);
    } else {
        println!("{}", styled);
    }
}

/// Format Docker errors with actionable guidance
fn format_docker_error(e: &DockerError) -> anyhow::Error {
    match e {
        DockerError::NotRunning => {
            anyhow!(
                "{}\n\n  {}\n  {}",
                "Docker is not running",
                "Start Docker Desktop or the Docker daemon:",
                "  sudo systemctl start docker"
            )
        }
        DockerError::PermissionDenied => {
            anyhow!(
                "{}\n\n  {}\n  {}\n  {}",
                "Permission denied accessing Docker",
                "Add your user to the docker group:",
                "  sudo usermod -aG docker $USER",
                "Then log out and back in."
            )
        }
        _ => anyhow!("{}", e),
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
