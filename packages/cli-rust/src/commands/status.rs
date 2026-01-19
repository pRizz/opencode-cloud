//! Status command implementation
//!
//! Shows the current state of the opencode service including container info,
//! port bindings, uptime, and health status.

use crate::output::state_style;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::config;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, DockerError, OPENCODE_WEB_PORT};
use opencode_cloud_core::platform::{get_service_manager, is_service_registration_supported};
use std::time::Duration;

/// Arguments for the status command
#[derive(Args)]
pub struct StatusArgs {}

/// Show the status of the opencode service
///
/// In normal mode, displays a key-value formatted status including:
/// - State (colored: green=running, red=stopped)
/// - URL (if running)
/// - Container name and ID
/// - Image name
/// - Uptime (if running)
/// - Port binding
/// - Health status (if available)
/// - Config file path
///
/// In quiet mode:
/// - Exits 0 if running
/// - Exits 1 if stopped
/// - No output
pub async fn cmd_status(_args: &StatusArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Connect to Docker
    let client = DockerClient::new().map_err(|e| format_docker_error(&e))?;

    // Verify connection
    client
        .verify_connection()
        .await
        .map_err(|e| format_docker_error(&e))?;

    // Check if container exists
    let inspect_result = client.inner().inspect_container(CONTAINER_NAME, None).await;

    let info = match inspect_result {
        Ok(info) => info,
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            if quiet {
                std::process::exit(1);
            }
            println!("{}", style("No service found.").yellow());
            println!();
            println!("Run '{}' to start the service.", style("occ start").cyan());
            return Ok(());
        }
        Err(e) => {
            return Err(anyhow!("Failed to inspect container: {}", e));
        }
    };

    // Extract state information
    let state = info.state.as_ref();
    let status = state
        .and_then(|s| s.status.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let running = state.and_then(|s| s.running).unwrap_or(false);
    let started_at = state.and_then(|s| s.started_at.clone());
    let finished_at = state.and_then(|s| s.finished_at.clone());
    let health = state
        .and_then(|s| s.health.as_ref())
        .and_then(|h| h.status.as_ref())
        .map(|s| s.to_string());

    // Extract container info
    let container_id = info.id.as_deref().unwrap_or("unknown");
    let id_short = &container_id[..12.min(container_id.len())];
    let image = info
        .config
        .as_ref()
        .and_then(|c| c.image.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract port binding
    let host_port = info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .and_then(|ports| ports.get("3000/tcp"))
        .and_then(|bindings| bindings.as_ref())
        .and_then(|bindings| bindings.first())
        .and_then(|binding| binding.host_port.as_ref())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(OPENCODE_WEB_PORT);

    // Quiet mode: just exit with appropriate code
    if quiet {
        if running {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    // Get config path
    let config_path = config::paths::get_config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Normal mode: print formatted status
    println!("State:       {}", state_style(&status));

    if running {
        let url = format!("http://127.0.0.1:{}", host_port);
        println!("URL:         {}", style(&url).cyan());
    }

    println!(
        "Container:   {} ({})",
        CONTAINER_NAME,
        style(id_short).dim()
    );
    println!("Image:       {}", image);

    if running {
        // Calculate and display uptime
        if let Some(ref started) = started_at {
            if let Some((uptime, started_display)) = parse_uptime(started) {
                let uptime_str = format_duration(uptime);
                println!("Uptime:      {} (since {})", uptime_str, started_display);
            }
        }

        println!(
            "Port:        {} -> container:3000",
            style(host_port.to_string()).cyan()
        );
    }

    // Show health if available
    if let Some(ref health_status) = health {
        let health_styled = match health_status.as_str() {
            "healthy" => style(health_status).green(),
            "unhealthy" => style(health_status).red(),
            "starting" => style(health_status).yellow(),
            _ => style(health_status).dim(),
        };
        println!("Health:      {}", health_styled);
    }

    println!("Config:      {}", style(&config_path).dim());

    // Show installation status
    if is_service_registration_supported() {
        if let Ok(manager) = get_service_manager() {
            let installed = manager.is_installed().unwrap_or(false);
            let install_status = if installed {
                // Load config to determine boot mode
                let boot_mode = config::load_config()
                    .map(|c| c.boot_mode)
                    .unwrap_or_else(|_| "user".to_string());
                let boot_desc = if boot_mode == "system" {
                    "starts on boot"
                } else {
                    "starts on login"
                };
                format!("{} ({})", style("yes").green(), boot_desc)
            } else {
                style("no").yellow().to_string()
            };
            println!("Installed:   {}", install_status);
        }
    }

    // If stopped, show when it stopped
    if !running {
        if let Some(ref finished) = finished_at {
            if let Some(display_time) = parse_timestamp_display(finished) {
                println!();
                println!("Last run:    {}", style(&display_time).dim());
            }
        }
        println!();
        println!("Run '{}' to start the service.", style("occ start").cyan());
    }

    Ok(())
}

/// Parse uptime from ISO8601 started_at timestamp
///
/// Returns (duration since start, human-readable start time) or None if parsing fails
fn parse_uptime(started_at: &str) -> Option<(Duration, String)> {
    // Docker timestamps are in format: "2024-01-15T10:30:00.123456789Z"
    // We need to handle this format and calculate uptime

    // Parse the timestamp - handle both with and without fractional seconds
    let timestamp = if started_at.contains('.') {
        // Has fractional seconds
        chrono::DateTime::parse_from_rfc3339(started_at).ok()?
    } else {
        // No fractional seconds - add .0 for parsing
        let fixed = started_at.replace('Z', ".0Z");
        chrono::DateTime::parse_from_rfc3339(&fixed).ok()?
    };

    let now = chrono::Utc::now();
    let started = timestamp.with_timezone(&chrono::Utc);

    if now < started {
        return None;
    }

    let duration = (now - started).to_std().ok()?;
    let display = started.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    Some((duration, display))
}

/// Parse timestamp for display (without calculating duration)
fn parse_timestamp_display(timestamp: &str) -> Option<String> {
    let ts = if timestamp.contains('.') {
        chrono::DateTime::parse_from_rfc3339(timestamp).ok()?
    } else {
        let fixed = timestamp.replace('Z', ".0Z");
        chrono::DateTime::parse_from_rfc3339(&fixed).ok()?
    };

    Some(ts.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

/// Format a duration in a human-readable way
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        return format!("{}s", secs);
    }

    let mins = secs / 60;
    if mins < 60 {
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            return format!("{}m {}s", mins, remaining_secs);
        }
        return format!("{}m", mins);
    }

    let hours = mins / 60;
    let remaining_mins = mins % 60;
    if hours < 24 {
        if remaining_mins > 0 {
            return format!("{}h {}m", hours, remaining_mins);
        }
        return format!("{}h", hours);
    }

    let days = hours / 24;
    let remaining_hours = hours % 24;
    if remaining_hours > 0 {
        return format!("{}d {}h", days, remaining_hours);
    }
    format!("{}d", days)
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
    fn format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(120)), "2m");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(format_duration(Duration::from_secs(3660)), "1h 1m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
    }

    #[test]
    fn format_duration_days() {
        assert_eq!(format_duration(Duration::from_secs(86400)), "1d");
        assert_eq!(format_duration(Duration::from_secs(90000)), "1d 1h");
    }

    #[test]
    fn parse_uptime_with_fractional_seconds() {
        // This test verifies the parsing logic works
        // The actual duration will vary based on current time
        let timestamp = "2024-01-15T10:30:00.123456789Z";
        let result = parse_uptime(timestamp);
        assert!(result.is_some());
        let (_, display) = result.unwrap();
        assert!(display.contains("2024-01-15"));
    }

    #[test]
    fn parse_uptime_without_fractional_seconds() {
        let timestamp = "2024-01-15T10:30:00Z";
        let result = parse_uptime(timestamp);
        assert!(result.is_some());
        let (_, display) = result.unwrap();
        assert!(display.contains("2024-01-15"));
    }

    #[test]
    fn parse_timestamp_display_works() {
        let timestamp = "2024-01-15T10:30:00.123Z";
        let result = parse_timestamp_display(timestamp);
        assert!(result.is_some());
        let display = result.unwrap();
        assert!(display.contains("2024-01-15"));
        assert!(display.contains("10:30:00"));
    }
}
