//! Container-mode status command implementation.

use crate::commands::container::{exec_command, exec_command_with_status, systemd_available};
use crate::output::state_style;
use anyhow::Result;
use console::style;
use opencode_cloud_core::docker::{HealthError, OPENCODE_WEB_PORT, check_health, get_cli_version};
use std::fs;
use std::path::Path;

const STATUS_LABEL_WIDTH: usize = 15;

enum OpencodeHealthStatus {
    Healthy,
    Starting,
    Unhealthy,
    Failed,
}

pub async fn cmd_status_container(
    _args: &crate::commands::StatusArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let systemd = systemd_available();

    let opencode_running = if systemd {
        systemd_service_active("opencode.service").await?
    } else {
        process_running("pgrep", &["-f", "/opt/opencode/bin/opencode"]).await?
    };

    if quiet {
        if opencode_running {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    let broker_running = if systemd {
        systemd_service_active("opencode-broker.service").await?
    } else {
        process_running("pgrep", &["-x", "opencode-broker"]).await?
    };

    let broker_socket = Path::new("/run/opencode/auth.sock").exists();

    let opencode_version = read_opencode_version().await;
    let opencode_commit = read_opencode_commit();
    let opencode_cloud_version = read_opencode_cloud_version();

    let health = if opencode_running {
        get_opencode_health_status(true).await
    } else {
        None
    };

    let state_label = if opencode_running {
        match health {
            Some(OpencodeHealthStatus::Healthy) | None => "running".to_string(),
            Some(OpencodeHealthStatus::Starting) => "starting".to_string(),
            Some(OpencodeHealthStatus::Unhealthy) => "unhealthy".to_string(),
            Some(OpencodeHealthStatus::Failed) => "unknown".to_string(),
        }
    } else {
        "stopped".to_string()
    };

    println!("{}", format_kv("State:", state_style(&state_label)));
    println!(
        "{}",
        format_kv(
            "URL:",
            style(format!("http://127.0.0.1:{OPENCODE_WEB_PORT}")).cyan()
        )
    );

    let opencode_display = match (opencode_version.as_deref(), opencode_commit.as_deref()) {
        (Some(version), Some(commit)) => format!("{version} ({commit})"),
        (Some(version), None) => version.to_string(),
        (None, Some(commit)) => format!("unknown ({commit})"),
        (None, None) => "unknown".to_string(),
    };
    println!("{}", format_kv("Opencode:", opencode_display));

    let broker_state = if broker_running { "running" } else { "stopped" };
    let socket_state = if broker_socket {
        "socket ok"
    } else {
        "socket missing"
    };
    println!(
        "{}",
        format_kv("Broker:", format!("{broker_state} ({socket_state})"))
    );

    let runtime = if systemd { "systemd" } else { "tini" };
    println!("{}", format_kv("Runtime:", runtime));

    let image_version = opencode_cloud_version.unwrap_or_else(|| "unknown".to_string());
    println!("{}", format_kv("Image:", image_version));

    let cli_version = get_cli_version();
    println!("{}", format_kv("CLI:", format!("v{cli_version}")));

    Ok(())
}

async fn systemd_service_active(service: &str) -> Result<bool> {
    let (output, _status) = exec_command_with_status("systemctl", &["is-active", service]).await?;
    Ok(output.lines().next().map(|line| line.trim()) == Some("active"))
}

async fn process_running(cmd: &str, args: &[&str]) -> Result<bool> {
    let (_output, status) = exec_command_with_status(cmd, args).await?;
    Ok(status == 0)
}

async fn read_opencode_version() -> Option<String> {
    let output = exec_command("/opt/opencode/bin/opencode", &["--version"])
        .await
        .ok()?;
    let version = output.lines().next()?.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn read_opencode_commit() -> Option<String> {
    let contents = fs::read_to_string("/opt/opencode/COMMIT").ok()?;
    let commit = contents.lines().next()?.trim();
    if commit.is_empty() {
        None
    } else {
        Some(commit.chars().take(7).collect())
    }
}

fn read_opencode_cloud_version() -> Option<String> {
    let contents = fs::read_to_string("/etc/opencode-cloud-version").ok()?;
    let version = contents.lines().next()?.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

async fn get_opencode_health_status(include_probe: bool) -> Option<OpencodeHealthStatus> {
    if !include_probe {
        return None;
    }

    match check_health("127.0.0.1", OPENCODE_WEB_PORT).await {
        Ok(_) => Some(OpencodeHealthStatus::Healthy),
        Err(HealthError::ConnectionRefused) | Err(HealthError::Timeout) => {
            Some(OpencodeHealthStatus::Starting)
        }
        Err(HealthError::Unhealthy(_code)) => Some(OpencodeHealthStatus::Unhealthy),
        Err(_) => Some(OpencodeHealthStatus::Failed),
    }
}

fn format_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label:<STATUS_LABEL_WIDTH$} {value}")
}
