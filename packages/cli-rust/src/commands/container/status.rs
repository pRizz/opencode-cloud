//! Container-mode status command implementation.

use crate::commands::container::{exec_command_with_status, systemd_available};
use crate::commands::runtime_shared::backend::{ContainerBackend, default_container_port};
use crate::commands::runtime_shared::collect_status_view;
use crate::commands::runtime_shared::status_model::{
    OpencodeHealthStatus, format_broker_health_label,
};
use crate::output::{format_service_url, state_style};
use anyhow::Result;
use console::style;
use opencode_cloud_core::docker::get_cli_version;

const STATUS_LABEL_WIDTH: usize = 15;

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

    let backend = ContainerBackend::new(systemd);
    let host_port = default_container_port();
    let status_view =
        collect_status_view(&backend, opencode_running, "127.0.0.1", host_port).await?;

    let state_label = if opencode_running {
        match status_view.opencode_health {
            Some(OpencodeHealthStatus::Healthy) | None => "running".to_string(),
            Some(OpencodeHealthStatus::Starting) => "starting".to_string(),
            Some(OpencodeHealthStatus::Unhealthy(_)) => "unhealthy".to_string(),
            Some(OpencodeHealthStatus::CheckFailed) => "unknown".to_string(),
        }
    } else {
        "stopped".to_string()
    };

    println!("{}", format_kv("State:", state_style(&state_label)));
    println!(
        "{}",
        format_kv(
            "URL:",
            style(format_service_url(None, "127.0.0.1", host_port)).cyan()
        )
    );

    let opencode_display =
        format_opencode_display(&status_view.opencode_version, &status_view.opencode_commit);
    println!("{}", format_kv("Opencode:", opencode_display));

    println!(
        "{}",
        format_kv(
            "Broker:",
            format_broker_health_label(status_view.broker_health)
        )
    );

    let runtime = if status_view
        .capabilities
        .systemd_available
        .unwrap_or(systemd)
    {
        "systemd"
    } else {
        "tini"
    };
    println!("{}", format_kv("Runtime:", runtime));
    println!("{}", format_kv("Image:", &status_view.image_version));

    let cli_version = get_cli_version();
    println!("{}", format_kv("CLI:", format!("v{cli_version}")));

    Ok(())
}

async fn systemd_service_active(service: &str) -> Result<bool> {
    let (_output, status) = exec_command_with_status("systemctl", &["is-active", service]).await?;
    Ok(status == 0)
}

async fn process_running(cmd: &str, args: &[&str]) -> Result<bool> {
    let (_output, status) = exec_command_with_status(cmd, args).await?;
    Ok(status == 0)
}

fn format_opencode_display(version: &str, commit: &str) -> String {
    match (version, commit) {
        ("unknown", "unknown") => "unknown".to_string(),
        ("unknown", commit) => format!("unknown ({commit})"),
        (version, "unknown") => version.to_string(),
        (version, commit) => format!("{version} ({commit})"),
    }
}

fn format_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label:<STATUS_LABEL_WIDTH$} {value}")
}
