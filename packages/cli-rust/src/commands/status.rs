//! Status command implementation
//!
//! Shows the current state of the opencode service including container info,
//! port bindings, uptime, health status, and security configuration.

use crate::cli_platform::cli_platform_label;
use crate::commands::disk_usage::{
    format_disk_usage_report, format_host_disk_report, get_disk_usage_report, get_host_disk_report,
};
use crate::commands::iotp::{IOTP_FALLBACK_COMMAND, IotpSnapshot, IotpState, fetch_iotp_snapshot};
use crate::commands::runtime_shared::backend::HostBackend;
use crate::commands::runtime_shared::collect_status_view;
use crate::commands::runtime_shared::drift::{
    RuntimeAssetDrift, detect_runtime_asset_drift, stale_container_warning_lines,
};
use crate::commands::runtime_shared::status_model::{
    BrokerHealthStatus, OpencodeHealthStatus, format_broker_health_label,
    format_opencode_health_label,
};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::{
    format_cockpit_url, format_docker_error_anyhow, format_service_url, resolve_remote_addr,
    state_style,
};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::Config;
use opencode_cloud_core::bollard::service::MountTypeEnum;
use opencode_cloud_core::config;
use opencode_cloud_core::docker::{
    MOUNT_CACHE, MOUNT_CONFIG, MOUNT_PROJECTS, MOUNT_SESSION, MOUNT_STATE, OPENCODE_WEB_PORT,
    ParsedMount, active_resource_names, get_cli_version, get_image_version, load_state,
};
use opencode_cloud_core::platform::{get_service_manager, is_service_registration_supported};
use std::collections::HashMap;
use std::time::Duration;

/// Arguments for the status command
#[derive(Args)]
pub struct StatusArgs {}

const STATUS_LABEL_WIDTH: usize = 15;

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
    verbose: u8,
) -> Result<()> {
    let resources = active_resource_names();

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
    let inspect_result = client
        .inner()
        .inspect_container(&resources.container_name, None)
        .await;

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

    // Extract port bindings
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

    // Load config early for reuse in multiple sections
    let config = config::load_config_or_default().ok();
    let bind_addr = config
        .as_ref()
        .map(|cfg| cfg.bind_address.as_str())
        .unwrap_or("127.0.0.1");

    // Get remote host address if using --remote-host
    let maybe_remote_addr = resolve_remote_addr(host_name.as_deref());

    // Normal mode: print formatted status
    println!("{}", format_kv("State:", state_style(&status)));

    // Show installation status early
    if is_service_registration_supported() {
        // Load config to determine boot mode
        let boot_mode = config::load_config_or_default()
            .map(|c| c.boot_mode)
            .unwrap_or_else(|_| "user".to_string());
        if let Ok(manager) = get_service_manager(&boot_mode) {
            let installed = manager.is_installed().unwrap_or(false);
            let install_status = if installed {
                let boot_desc = if boot_mode == "system" {
                    "starts on boot"
                } else {
                    "starts on login"
                };
                format!("{} ({})", style("yes").green(), boot_desc)
            } else {
                style("no").yellow().to_string()
            };
            println!("{}", format_kv("Installed:", install_status));
        }
    }

    // Label config path - clarify it's local config when using remote host
    print_config_path(host_name.as_deref(), &config_path);

    if running {
        print_section_header("OpenCode");
        let broker_health = print_opencode_section(
            &client,
            host_name.as_deref(),
            maybe_remote_addr.as_deref(),
            bind_addr,
            host_port,
            started_at.as_deref(),
        )
        .await?;

        print_section_header("OpenCode Broker");
        print_opencode_broker_section(broker_health);
    }

    print_section_header("Sandbox");
    let container_id = format!("({id_short})");
    println!(
        "{}",
        format_kv(
            "Container:",
            format!("{} {}", resources.container_name, style(container_id).dim())
        )
    );
    if let Some(instance_id) = resources.instance_id.as_deref() {
        println!("{}", format_kv("Instance:", instance_id));
    }
    println!("{}", format_kv("Image:", &image));

    // Show CLI and image versions
    let cli_version = get_cli_version();
    let cli_label = format!("{}:", cli_platform_label());
    println!("{}", format_kv(&cli_label, format!("v{cli_version}")));

    // Try to get image version from label
    if let Ok(Some(img_version)) = get_image_version(&client, &image).await
        && img_version != "dev"
    {
        if cli_version == img_version {
            println!("{}", format_kv("Image ver:", format!("v{img_version}")));
        } else {
            println!(
                "{}",
                format_kv(
                    "Image ver:",
                    format!(
                        "v{} {}",
                        img_version,
                        style(format!("(differs from {})", cli_platform_label()))
                            .yellow()
                            .dim()
                    )
                )
            );
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
        println!("{}", format_kv("Image src:", style(&source_info).dim()));
    }

    let runtime_asset_drift = if running && host_name.is_none() {
        detect_runtime_asset_drift(&client).await
    } else {
        RuntimeAssetDrift::default()
    };
    print_runtime_asset_drift_warning(&runtime_asset_drift, verbose);

    print_disk_usage_section(&client, host_name.as_deref()).await;

    if running {
        print_cockpit(
            maybe_remote_addr.as_deref(),
            host_name.as_deref(),
            config.as_ref(),
        );
    }

    if host_name.is_some() {
        print_remote_health(health.as_deref());
    }

    let config_mounts = config
        .as_ref()
        .map(|c| c.mounts.clone())
        .unwrap_or_default();
    let volume_mountpoints = resolve_volume_mountpoints(&client, &container_mounts).await;
    display_mounts_section(&container_mounts, &config_mounts, &volume_mountpoints);

    // Show Security section (container exists, whether running or stopped)
    if let Some(ref cfg) = config {
        display_security_section(&client, cfg, running).await;
    }

    // If stopped, show when it stopped
    if !running {
        print_stopped_section(finished_at.as_deref());
    }

    Ok(())
}

async fn print_disk_usage_section(
    client: &opencode_cloud_core::docker::DockerClient,
    maybe_host_name: Option<&str>,
) {
    print_section_header("Disk");
    match get_disk_usage_report(client).await {
        Ok(report) => {
            for line in format_disk_usage_report("current", report, None) {
                println!("{line}");
            }
        }
        Err(err) => {
            println!("{} {err}", style("Warning:").yellow().bold());
        }
    }

    match get_host_disk_report(client) {
        Ok(Some(report)) => {
            println!();
            for line in format_host_disk_report("current", report, None) {
                println!("{line}");
            }
        }
        Ok(None) => {
            if maybe_host_name.is_some() {
                println!(
                    "{}",
                    format_kv(
                        "Note:",
                        style("Host disk stats unavailable for remote Docker hosts.").dim()
                    )
                );
            }
        }
        Err(err) => {
            println!("{} {err}", style("Warning:").yellow().bold());
        }
    }
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

fn format_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("{} {}", format_label(label), value)
}

fn format_label(label: &str) -> String {
    format!("{label:<STATUS_LABEL_WIDTH$}")
}

fn format_continuation(value: impl std::fmt::Display) -> String {
    format!("{:width$}{}", "", value, width = STATUS_LABEL_WIDTH + 1)
}

fn print_section_header(title: &str) {
    println!();
    println!("{}", style(title).bold());
    println!("{}", style("------").dim());
}

async fn print_opencode_section(
    client: &opencode_cloud_core::docker::DockerClient,
    maybe_host_name: Option<&str>,
    maybe_remote_addr: Option<&str>,
    bind_addr: &str,
    host_port: u16,
    started_at: Option<&str>,
) -> Result<BrokerHealthStatus> {
    let backend = HostBackend::new(client);
    let status_view =
        collect_status_view(&backend, maybe_host_name.is_none(), bind_addr, host_port).await?;

    print_urls(maybe_remote_addr, bind_addr, host_port);

    if let Some(health_status) = status_view.opencode_health {
        print_opencode_health(health_status);
    }

    print_opencode_version_commit(&status_view.opencode_version, &status_view.opencode_commit);
    print_uptime(started_at);
    print_port(host_port);

    Ok(status_view.broker_health)
}

fn print_opencode_broker_section(status: BrokerHealthStatus) {
    print_broker_health_line(status);
}

fn print_urls(maybe_remote_addr: Option<&str>, bind_addr: &str, host_port: u16) {
    let Some(remote_addr) = maybe_remote_addr else {
        let url = format_service_url(None, bind_addr, host_port);
        println!("{}", format_kv("Local URL:", style(&url).cyan()));
        return;
    };

    let remote_url = format_service_url(Some(remote_addr), bind_addr, host_port);
    println!("{}", format_kv("Remote URL:", style(&remote_url).cyan()));
    let local_url = format_service_url(None, "127.0.0.1", host_port);
    println!(
        "{}",
        format_kv(
            "Local URL:",
            format!(
                "{} {}",
                style(&local_url).dim(),
                style("(on remote host)").dim()
            )
        )
    );
}

fn print_uptime(started_at: Option<&str>) {
    let Some(started) = started_at else {
        return;
    };
    let Some((uptime, started_display)) = parse_uptime(started) else {
        return;
    };
    let uptime_str = format_duration(uptime);
    println!(
        "{}",
        format_kv("Uptime:", format!("{uptime_str} (since {started_display})"))
    );
}

fn print_port(host_port: u16) {
    println!(
        "{}",
        format_kv(
            "Port:",
            format!("{} -> container:3000", style(host_port.to_string()).cyan())
        )
    );
}

fn print_cockpit(
    maybe_remote_addr: Option<&str>,
    maybe_host_name: Option<&str>,
    config: Option<&Config>,
) {
    let Some(cfg) = config else {
        return;
    };
    if !cfg.cockpit_enabled {
        return;
    }
    if !COCKPIT_EXPOSED {
        return;
    }

    let cockpit_url = format_cockpit_url(maybe_remote_addr, &cfg.bind_address, cfg.cockpit_port);
    println!(
        "{}",
        format_kv(
            "Cockpit:",
            format!("{} -> container:9090", style(&cockpit_url).cyan())
        )
    );
    let user_cmd = if let Some(name) = maybe_host_name {
        format!("occ user add <username> --remote-host {name}")
    } else {
        "occ user add <username>".to_string()
    };
    println!(
        "{}",
        format_continuation(style("Cockpit authenticates against container system users.").dim())
    );
    println!(
        "{}",
        format_continuation(format!(
            "{} {}",
            style("Create a container user with:").dim(),
            style(&user_cmd).cyan()
        ))
    );
}

fn print_remote_health(maybe_health: Option<&str>) {
    let Some(health_status) = maybe_health else {
        return;
    };

    let health_styled = match health_status {
        "healthy" => style(health_status).green(),
        "unhealthy" => style(health_status).red(),
        "starting" => style(health_status).yellow(),
        _ => style(health_status).dim(),
    };
    println!("{}", format_kv("Sandbox Health:", health_styled));
}

fn print_runtime_asset_drift_warning(report: &RuntimeAssetDrift, verbose: u8) {
    if !report.drift_detected {
        return;
    }

    println!(
        "{}",
        format_kv(
            "Dev sync:",
            style("stale container assets detected").yellow()
        )
    );
    for line in render_runtime_asset_drift_lines(report, verbose) {
        println!("{}", format_continuation(style(line).yellow()));
    }
}

fn render_runtime_asset_drift_lines(report: &RuntimeAssetDrift, verbose: u8) -> Vec<String> {
    let mut lines = stale_container_warning_lines(report);
    if verbose > 0 {
        for detail in &report.diagnostics {
            lines.push(format!("diagnostic: {detail}"));
        }
    }
    lines
}

fn print_config_path(maybe_host_name: Option<&str>, config_path: &str) {
    let label = if maybe_host_name.is_some() {
        "Local Config:"
    } else {
        "Config:"
    };
    println!("{}", format_kv(label, style(config_path).dim()));
}

fn print_stopped_section(finished_at: Option<&str>) {
    if let Some(finished) = finished_at
        && let Some(display_time) = parse_timestamp_display(finished)
    {
        println!();
        println!("{}", format_kv("Last run:", style(&display_time).dim()));
    }
    println!();
    println!("Run '{}' to start the service.", style("occ start").cyan());
}

fn print_opencode_health(status: OpencodeHealthStatus) {
    let value = format_opencode_health_label(status);
    println!("{}", format_kv("Health:", value));
}

fn print_broker_health_line(status: BrokerHealthStatus) {
    let value = format_broker_health_label(status);
    println!("{}", format_kv("Health:", value));
}

fn print_opencode_version_commit(version: &str, commit: &str) {
    println!("{}", format_kv("Version:", version));
    println!("{}", format_kv("Commit:", commit));

    if commit != "unknown" {
        let repo_url = format!("https://github.com/pRizz/opencode/commit/{commit}");
        println!("{}", format_kv("Commit link:", style(&repo_url).cyan()));
    }
}

/// Display the Mounts section of status output
fn display_mounts_section(
    mounts: &[opencode_cloud_core::bollard::service::Mount],
    config_mounts: &[String],
    volume_mountpoints: &HashMap<String, String>,
) {
    let volume_mounts: Vec<_> = mounts
        .iter()
        .filter(|m| m.typ == Some(MountTypeEnum::VOLUME))
        .collect();
    let bind_mounts: Vec<_> = mounts
        .iter()
        .filter(|m| m.typ == Some(MountTypeEnum::BIND))
        .collect();

    if volume_mounts.is_empty() && bind_mounts.is_empty() {
        return;
    }

    println!();
    println!("{}", style("Mounts").bold());
    println!("{}", style("------").dim());
    print_mounts_hint();

    // Parse config mounts for source detection
    let config_parsed: Vec<ParsedMount> = config_mounts
        .iter()
        .filter_map(|m| ParsedMount::parse(m).ok())
        .collect();

    if volume_mounts.is_empty() {
        println!("  Volumes: {}", style("(none)").dim());
    } else {
        println!("  Volumes:");
        for mount in volume_mounts {
            let source = mount.source.as_deref().unwrap_or("unknown");
            let target = mount.target.as_deref().unwrap_or("unknown");
            let source_path = volume_mountpoints
                .get(source)
                .map(String::as_str)
                .unwrap_or(source);
            let mode = if mount.read_only.unwrap_or(false) {
                "ro"
            } else {
                "rw"
            };
            let name_tag = if source_path == source {
                ""
            } else {
                " (volume)"
            };
            let purpose = mount_purpose(target);
            let annotation = purpose
                .map(|value| format!(" - {value}"))
                .unwrap_or_default();
            println!(
                "    {} -> {} {}{}{}",
                style(source_path).cyan(),
                target,
                style(mode).dim(),
                style(name_tag).dim(),
                style(annotation).dim()
            );
            println!(
                "      {}",
                style(format!(
                    "docker run --rm -it -v {source}:/data -w /data alpine sh"
                ))
                .dim()
            );
        }
    }

    if bind_mounts.is_empty() {
        println!("  Bind mounts: {}", style("(none)").dim());
    } else {
        println!("  Bind mounts:");
        for mount in bind_mounts {
            let source = mount.source.as_deref().unwrap_or("unknown");
            let target = mount.target.as_deref().unwrap_or("unknown");
            let display_source = format_bind_source_for_display(source);
            let mode = if mount.read_only.unwrap_or(false) {
                "ro"
            } else {
                "rw"
            };

            // Determine if this mount came from config or CLI
            // Needs path matching to handle macOS translation (/tmp -> /host_mnt/private/tmp)
            // Must match both source AND target paths to be considered from config
            let is_from_config = config_parsed.iter().any(|conf| {
                let conf_host = conf.host_path.to_string_lossy();
                host_paths_match(source, &conf_host) && target == conf.container_path
            });
            let source_tag = if is_from_config {
                style("(config)").dim()
            } else {
                style("(cli)").cyan()
            };
            let purpose = mount_purpose(target);
            let annotation = purpose
                .map(|value| format!(" - {value}"))
                .unwrap_or_default();

            println!(
                "    {} -> {} {} {}{}",
                style(display_source.as_ref()).cyan(),
                target,
                style(mode).dim(),
                source_tag,
                style(annotation).dim()
            );
        }
    }
}

fn print_mounts_hint() {
    let instruction = if std::env::consts::OS == "macos" {
        "Hint: Docker Desktop stores volumes inside its VM. Use `docker run --rm -it -v <volume>:/data -w /data alpine sh` to inspect volumes."
    } else {
        "Hint: Inspect volumes with: docker run --rm -it -v <volume>:/data -w /data alpine sh"
    };
    println!("{}", style(instruction).dim());
}

fn mount_purpose(target: &str) -> Option<&'static str> {
    match target {
        MOUNT_SESSION => Some("opencode data"),
        MOUNT_STATE => Some("opencode state"),
        MOUNT_CACHE => Some("opencode cache"),
        MOUNT_PROJECTS => Some("workspace files"),
        MOUNT_CONFIG => Some("opencode config"),
        _ => None,
    }
}

fn format_bind_source_for_display(source: &str) -> std::borrow::Cow<'_, str> {
    if std::env::consts::OS != "macos" {
        return std::borrow::Cow::Borrowed(source);
    }

    // Docker Desktop for macOS reports bind sources using /host_mnt
    // inside its Linux VM; strip it to show the host path.
    if let Some(stripped) = source.strip_prefix("/host_mnt") {
        return std::borrow::Cow::Owned(stripped.to_string());
    }

    std::borrow::Cow::Borrowed(source)
}

async fn resolve_volume_mountpoints(
    client: &opencode_cloud_core::docker::DockerClient,
    mounts: &[opencode_cloud_core::bollard::service::Mount],
) -> HashMap<String, String> {
    let volume_names: Vec<String> = mounts
        .iter()
        .filter(|m| m.typ == Some(MountTypeEnum::VOLUME))
        .filter_map(|m| m.source.clone())
        .collect();

    let mut resolved = HashMap::new();
    for name in volume_names {
        if resolved.contains_key(&name) {
            continue;
        }
        if let Ok(info) = client.inner().inspect_volume(&name).await {
            resolved.insert(name.clone(), info.mountpoint);
        }
    }

    resolved
}

/// Check if two host paths match, accounting for macOS path translation
///
/// Docker on macOS translates paths: /tmp -> /private/tmp -> /host_mnt/private/tmp
fn host_paths_match(container_path: &str, configured_path: &str) -> bool {
    // Direct match
    if container_path == configured_path {
        return true;
    }

    // Handle /host_mnt prefix from Docker Desktop
    if let Some(stripped) = container_path.strip_prefix("/host_mnt") {
        if stripped == configured_path {
            return true;
        }
        // /host_mnt/private/tmp matches /tmp
        if let Some(private_stripped) = stripped.strip_prefix("/private")
            && private_stripped == configured_path
        {
            return true;
        }
    }

    // Handle /private prefix (macOS symlink: /tmp -> /private/tmp)
    if let Some(private_path) = configured_path.strip_prefix("/private")
        && container_path.ends_with(private_path)
    {
        return true;
    }

    false
}

/// Display the Security section of status output
async fn display_security_section(
    client: &opencode_cloud_core::docker::DockerClient,
    config: &Config,
    running: bool,
) {
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

    let iotp_snapshot = if running {
        fetch_iotp_snapshot(client).await
    } else {
        IotpSnapshot::unavailable("container not running")
    };
    let (iotp_state, iotp_value, iotp_detail) = render_iotp_status(
        &iotp_snapshot,
        running,
        config.allow_unauthenticated_network,
    );
    println!("IOTP state:  {iotp_state}");
    println!("IOTP value:  {iotp_value}");
    if let Some(detail) = iotp_detail {
        println!("{}", format_continuation(style(detail).dim()));
    }
    if matches!(iotp_snapshot.state, IotpState::InactiveCompleted) {
        for line in iotp_reset_hint_lines(!config.is_localhost()) {
            println!(
                "{}",
                format_continuation(format_iotp_reset_hint_line(&line))
            );
        }
    }
    if !matches!(iotp_snapshot.state, IotpState::ActiveUnused) {
        println!(
            "{}",
            format_continuation(format!(
                "{} {}",
                style("Extract manually:").dim(),
                style(IOTP_FALLBACK_COMMAND).cyan()
            ))
        );
    }

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

fn render_iotp_status(
    snapshot: &IotpSnapshot,
    running: bool,
    allow_unauthenticated_network: bool,
) -> (String, String, Option<String>) {
    let state_text = if running {
        snapshot.state_label.clone()
    } else {
        "unavailable (container not running)".to_string()
    };

    let value_text = if running && matches!(snapshot.state, IotpState::ActiveUnused) {
        snapshot
            .otp
            .clone()
            .unwrap_or_else(|| "not available".to_string())
    } else {
        style("not available").dim().to_string()
    };

    let mut detail = snapshot.detail.clone();
    if allow_unauthenticated_network {
        let note = "allow_unauthenticated_network=true may skip IOTP generation by design.";
        detail = Some(match detail {
            Some(existing) => format!("{existing} {note}"),
            None => note.to_string(),
        });
    }

    (state_text, value_text, detail)
}

fn iotp_reset_hint_lines(non_localhost_bind: bool) -> Vec<String> {
    let mut lines = vec!["Reset IOTP: occ reset iotp".to_string()];
    if non_localhost_bind {
        lines.push("Use --force for exposed bind configurations.".to_string());
    }
    lines
}

fn format_iotp_reset_hint_line(line: &str) -> String {
    if let Some(command) = line.strip_prefix("Reset IOTP: ") {
        return format!("{} {}", style("Reset IOTP:").dim(), style(command).cyan());
    }
    style(line).dim().to_string()
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

    #[test]
    fn render_iotp_status_active_shows_value() {
        let snapshot = IotpSnapshot {
            state: IotpState::ActiveUnused,
            state_label: "unused (active)".to_string(),
            otp: Some("abc123".to_string()),
            detail: None,
        };
        let (state, value, detail) = render_iotp_status(&snapshot, true, false);
        assert_eq!(state, "unused (active)");
        assert_eq!(value, "abc123");
        assert!(detail.is_none());
    }

    #[test]
    fn render_iotp_status_inactive_hides_value() {
        let snapshot = IotpSnapshot {
            state: IotpState::InactiveUsersConfigured,
            state_label: "inactive (users configured)".to_string(),
            otp: None,
            detail: None,
        };
        let (state, value, _detail) = render_iotp_status(&snapshot, true, false);
        assert_eq!(state, "inactive (users configured)");
        assert!(value.contains("not available"));
    }

    #[test]
    fn render_iotp_status_not_running_marks_unavailable() {
        let snapshot = IotpSnapshot::unavailable("container not running");
        let (state, value, detail) = render_iotp_status(&snapshot, false, false);
        assert_eq!(state, "unavailable (container not running)");
        assert!(value.contains("not available"));
        assert_eq!(detail, Some("container not running".to_string()));
    }

    #[test]
    fn render_iotp_status_adds_allow_unauth_note() {
        let snapshot = IotpSnapshot {
            state: IotpState::InactiveNotInitialized,
            state_label: "inactive (not initialized)".to_string(),
            otp: None,
            detail: Some("bootstrap helper reported reason: not_initialized".to_string()),
        };
        let (_state, _value, detail) = render_iotp_status(&snapshot, true, true);
        let detail = detail.expect("detail should exist");
        assert!(detail.contains("not_initialized"));
        assert!(detail.contains("allow_unauthenticated_network=true"));
    }

    #[test]
    fn iotp_reset_hint_lines_includes_force_hint_when_exposed() {
        let lines = iotp_reset_hint_lines(true);
        assert_eq!(lines[0], "Reset IOTP: occ reset iotp");
        assert!(lines.iter().any(|line| line.contains("--force")));
    }

    #[test]
    fn iotp_reset_hint_lines_without_exposed_only_shows_command() {
        let lines = iotp_reset_hint_lines(false);
        assert_eq!(lines, vec!["Reset IOTP: occ reset iotp".to_string()]);
    }

    #[test]
    fn render_runtime_asset_drift_lines_empty_when_no_drift() {
        let report = RuntimeAssetDrift::default();
        let lines = render_runtime_asset_drift_lines(&report, 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn render_runtime_asset_drift_lines_includes_rebuild_commands() {
        let report = RuntimeAssetDrift {
            drift_detected: true,
            mismatched_assets: vec!["bootstrap helper".to_string()],
            diagnostics: vec![],
        };
        let lines = render_runtime_asset_drift_lines(&report, 0);
        assert!(lines.iter().any(|line| line.contains("bootstrap helper")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("--cached-rebuild-sandbox-image"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("--full-rebuild-sandbox-image"))
        );
    }

    #[test]
    fn render_runtime_asset_drift_lines_includes_diagnostics_when_verbose() {
        let report = RuntimeAssetDrift {
            drift_detected: true,
            mismatched_assets: vec!["entrypoint".to_string()],
            diagnostics: vec!["healthcheck: exec failed".to_string()],
        };
        let lines = render_runtime_asset_drift_lines(&report, 1);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("diagnostic: healthcheck"))
        );
    }
}
