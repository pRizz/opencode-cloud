//! Status command implementation
//!
//! Shows the current state of the opencode service including container info,
//! port bindings, uptime, health status, and security configuration.

use crate::output::{
    format_cockpit_url, format_docker_error_anyhow, resolve_remote_addr, state_style,
};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::Config;
use opencode_cloud_core::config;
use opencode_cloud_core::bollard::service::MountTypeEnum;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, HealthError, OPENCODE_WEB_PORT, ParsedMount, check_health, get_cli_version,
    get_image_version, load_state,
};
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
pub async fn cmd_status(
    _args: &StatusArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    // Verify connection
    client
        .verify_connection()
        .await
        .map_err(|e| format_docker_error_anyhow(&e))?;

    // Show host header if remote
    if !quiet && host_name.is_some() {
        println!(
            "{}",
            crate::format_host_message(host_name.as_deref(), "Status")
        );
        println!();
    }

    // Check if container exists
    let inspect_result = client.inner().inspect_container(CONTAINER_NAME, None).await;

    let info = match inspect_result {
        Ok(info) => info,
        Err(opencode_cloud_core::bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
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
            return Err(anyhow!("Failed to inspect container: {e}"));
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

    // Extract bind mounts from container
    let container_mounts = info
        .host_config
        .as_ref()
        .and_then(|hc| hc.mounts.clone())
        .unwrap_or_default();

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

    // Get remote host address if using --host
    let maybe_remote_addr = resolve_remote_addr(host_name.as_deref());

    // Normal mode: print formatted status
    println!("State:       {}", state_style(&status));

    if running {
        // For remote hosts, show both container-local and remote-accessible URLs
        if let Some(ref remote_addr) = maybe_remote_addr {
            let remote_url = format!("http://{remote_addr}:{host_port}");
            println!("Remote URL:  {}", style(&remote_url).cyan());
            let local_url = format!("http://127.0.0.1:{host_port}");
            println!(
                "Local URL:   {} {}",
                style(&local_url).dim(),
                style("(on remote host)").dim()
            );
        } else {
            let url = format!("http://127.0.0.1:{host_port}");
            println!("URL:         {}", style(&url).cyan());
        }

        // Show health check status (only for local connections - can't check remote health directly)
        if host_name.is_none() {
            match check_health(host_port).await {
                Ok(response) => {
                    println!(
                        "Health:      {} (v{})",
                        style("Healthy").green(),
                        response.version
                    );
                }
                Err(HealthError::ConnectionRefused) | Err(HealthError::Timeout) => {
                    println!("Health:      {}", style("Service starting...").yellow());
                }
                Err(HealthError::Unhealthy(code)) => {
                    println!("Health:      {} (HTTP {})", style("Unhealthy").red(), code);
                }
                Err(_) => {
                    println!("Health:      {}", style("Check failed").yellow());
                }
            }
        }
    }

    println!(
        "Container:   {} ({})",
        CONTAINER_NAME,
        style(id_short).dim()
    );
    println!("Image:       {image}");

    // Show CLI and image versions
    let cli_version = get_cli_version();
    println!("CLI:         v{cli_version}");

    // Try to get image version from label
    if let Ok(Some(img_version)) = get_image_version(&client, &image).await {
        if img_version != "dev" {
            if cli_version == img_version {
                println!("Image ver:   v{img_version}");
            } else {
                println!(
                    "Image ver:   v{} {}",
                    img_version,
                    style("(differs from CLI)").yellow().dim()
                );
            }
        }
    }

    // Show image provenance from state file
    if let Some(state) = load_state() {
        let source_info = if state.source == "prebuilt" {
            if let Some(ref registry) = state.registry {
                format!("prebuilt from {registry}")
            } else {
                "prebuilt".to_string()
            }
        } else {
            "built from source".to_string()
        };
        println!("Image src:   {}", style(&source_info).dim());
    }

    // Load config early for reuse in multiple sections
    let config = config::load_config().ok();

    if running {
        // Calculate and display uptime
        if let Some(ref started) = started_at {
            if let Some((uptime, started_display)) = parse_uptime(started) {
                let uptime_str = format_duration(uptime);
                println!("Uptime:      {uptime_str} (since {started_display})");
            }
        }

        println!(
            "Port:        {} -> container:3000",
            style(host_port.to_string()).cyan()
        );

        // Show Cockpit info if enabled
        if let Some(ref cfg) = config {
            if cfg.cockpit_enabled {
                let cockpit_url = format_cockpit_url(
                    maybe_remote_addr.as_deref(),
                    &cfg.bind_address,
                    cfg.cockpit_port,
                );
                println!(
                    "Cockpit:     {} -> container:9090",
                    style(&cockpit_url).cyan()
                );
                // Show tip about creating users for Cockpit login
                let user_cmd = if let Some(ref name) = host_name {
                    format!("occ user add <username> --host {name}")
                } else {
                    "occ user add <username>".to_string()
                };
                println!(
                    "             {}",
                    style("Cockpit authenticates against container system users.").dim()
                );
                println!(
                    "             {} {}",
                    style("Create a container user with:").dim(),
                    style(&user_cmd).cyan()
                );
            }
        }
    }

    // Show health if available
    if let Some(ref health_status) = health {
        let health_styled = match health_status.as_str() {
            "healthy" => style(health_status).green(),
            "unhealthy" => style(health_status).red(),
            "starting" => style(health_status).yellow(),
            _ => style(health_status).dim(),
        };
        println!("Health:      {health_styled}");
    }

    // Label config path - clarify it's local config when using remote host
    if host_name.is_some() {
        println!("Local Config: {}", style(&config_path).dim());
    } else {
        println!("Config:      {}", style(&config_path).dim());
    }

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
            println!("Installed:   {install_status}");
        }
    }

    // Show Mounts section if container is running and has bind mounts
    if running {
        let config_mounts = config
            .as_ref()
            .map(|c| c.mounts.clone())
            .unwrap_or_default();
        display_mounts_section(&container_mounts, &config_mounts);
    }

    // Show Security section (container exists, whether running or stopped)
    if let Some(ref cfg) = config {
        display_security_section(cfg);
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
        return format!("{secs}s");
    }

    let mins = secs / 60;
    if mins < 60 {
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            return format!("{mins}m {remaining_secs}s");
        }
        return format!("{mins}m");
    }

    let hours = mins / 60;
    let remaining_mins = mins % 60;
    if hours < 24 {
        if remaining_mins > 0 {
            return format!("{hours}h {remaining_mins}m");
        }
        return format!("{hours}h");
    }

    let days = hours / 24;
    let remaining_hours = hours % 24;
    if remaining_hours > 0 {
        return format!("{days}d {remaining_hours}h");
    }
    format!("{days}d")
}

/// Display the Mounts section of status output
fn display_mounts_section(
    mounts: &[opencode_cloud_core::bollard::service::Mount],
    config_mounts: &[String],
) {
    // Filter to only bind mounts (not volumes)
    let bind_mounts: Vec<_> = mounts
        .iter()
        .filter(|m| m.typ == Some(MountTypeEnum::BIND))
        .collect();

    if bind_mounts.is_empty() {
        return;
    }

    println!();
    println!("{}", style("Mounts").bold());
    println!("{}", style("------").dim());

    // Create a set of config mount sources for source detection
    let config_sources: std::collections::HashSet<String> = config_mounts
        .iter()
        .filter_map(|m| {
            ParsedMount::parse(m)
                .ok()
                .map(|p| p.host_path.to_string_lossy().to_string())
        })
        .collect();

    for mount in bind_mounts {
        let source = mount.source.as_deref().unwrap_or("unknown");
        let target = mount.target.as_deref().unwrap_or("unknown");
        let mode = if mount.read_only.unwrap_or(false) {
            "ro"
        } else {
            "rw"
        };

        // Determine if this mount came from config or CLI
        let source_tag = if config_sources.contains(source) {
            style("(config)").dim()
        } else {
            style("(cli)").cyan()
        };

        println!(
            "  {} -> {} {} {}",
            style(source).cyan(),
            target,
            style(mode).dim(),
            source_tag
        );
    }
}

/// Display the Security section of status output
fn display_security_section(config: &Config) {
    println!();
    println!("{}", style("Security").bold());
    println!("{}", style("--------").dim());

    // Binding with badge
    let bind_badge = if config.is_network_exposed() {
        style("[NETWORK EXPOSED]").yellow().bold().to_string()
    } else {
        style("[LOCAL ONLY]").green().to_string()
    };
    println!(
        "Binding:     {} {}",
        style(&config.bind_address).cyan(),
        bind_badge
    );

    // Auth users list
    if config.users.is_empty() {
        println!("Auth users:  {}", style("None configured").yellow());
    } else {
        let users_list = config.users.join(", ");
        println!("Auth users:  {users_list}");
    }

    // Trust proxy
    let trust_proxy_str = if config.trust_proxy { "yes" } else { "no" };
    println!("Trust proxy: {trust_proxy_str}");

    // Rate limit
    println!(
        "Rate limit:  {} attempts / {}s window",
        config.rate_limit_attempts, config.rate_limit_window_seconds
    );

    // Warning if network exposed without users
    if config.is_network_exposed()
        && config.users.is_empty()
        && !config.allow_unauthenticated_network
    {
        println!();
        println!(
            "{}",
            style("Warning: Network exposed without authentication!")
                .yellow()
                .bold()
        );
        println!("Add users: {}", style("occ user add").cyan());
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
