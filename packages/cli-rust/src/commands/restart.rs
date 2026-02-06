//! Restart command implementation
//!
//! Restarts the opencode service (stop + start).

use crate::commands::runtime_shared::mounts::{collect_bind_mounts, mounts_equal};
use crate::commands::start::{wait_for_broker_ready, wait_for_service_ready};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::{CommandSpinner, format_docker_error, show_docker_error};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, ContainerBindMount, ParsedMount, container_exists, container_is_running,
    docker_supports_systemd, get_container_bind_mounts, setup_and_start, stop_service,
};

/// Arguments for the restart command
#[derive(Args)]
pub struct RestartArgs {
    // Future: --port flag to change port on restart
}

#[derive(Debug, PartialEq, Eq)]
enum MountMismatchAction {
    NoMismatch,
    PromptRecreate,
    QuietError(String),
}

fn resolve_mount_mismatch_action(
    current: &[ContainerBindMount],
    configured: &[ParsedMount],
    quiet: bool,
) -> MountMismatchAction {
    if mounts_equal(current, configured) {
        return MountMismatchAction::NoMismatch;
    }

    if quiet {
        return MountMismatchAction::QuietError(
            "Mount configuration changed. Container must be recreated to apply mount changes.\n\
             Run without --quiet to be prompted, or manually recreate with:\n  \
             occ stop --remove\n  \
             occ start"
                .to_string(),
        );
    }

    MountMismatchAction::PromptRecreate
}

fn display_mount_mismatch(current: &[ContainerBindMount], configured: &[ParsedMount]) {
    eprintln!();
    eprintln!(
        "{} {}",
        style("Mount configuration changed:").yellow().bold(),
        style("Container must be recreated to apply mount changes.").yellow()
    );
    eprintln!();

    display_current_mounts(current);
    display_configured_mounts(configured);

    eprintln!();
    eprintln!(
        "{}",
        style("This will stop and recreate the container from the existing image.").dim()
    );
    eprintln!("{}", style("Your data volumes will be preserved.").dim());
    eprintln!();
}

fn display_current_mounts(mounts: &[ContainerBindMount]) {
    if mounts.is_empty() {
        eprintln!("  Current mounts: {}", style("(none)").dim());
        return;
    }

    eprintln!("  Current mounts:");
    for mount in mounts {
        let ro = if mount.read_only { ":ro" } else { "" };
        eprintln!("    - {}:{}{}", mount.source, mount.target, ro);
    }
}

fn display_configured_mounts(mounts: &[ParsedMount]) {
    if mounts.is_empty() {
        eprintln!("  Configured mounts: {}", style("(none)").dim());
        return;
    }

    eprintln!("  Configured mounts:");
    for mount in mounts {
        let ro = if mount.read_only { ":ro" } else { "" };
        eprintln!(
            "    - {}:{}{}",
            mount.host_path.display(),
            mount.container_path,
            ro
        );
    }
}

/// Restart the opencode service
///
/// This command:
/// 1. Connects to Docker
/// 2. Stops the service if running
/// 3. Starts the service
pub async fn cmd_restart(
    _args: &RestartArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    if verbose > 0 {
        let target = host_name.as_deref().unwrap_or("local");
        eprintln!(
            "{} Connecting to Docker on {}...",
            style("[info]").cyan(),
            target
        );
    }

    // Verify connection
    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{msg}")
    })?;

    // Load config for port and bind_address
    let config = load_config_or_default()?;
    let port = config.opencode_web_port;
    let bind_addr = &config.bind_address;
    let systemd_enabled = docker_supports_systemd(&client).await?;
    let bind_mounts = collect_bind_mounts(&config, &[], false, quiet)?;
    let bind_mounts_option = if bind_mounts.is_empty() {
        None
    } else {
        Some(bind_mounts.clone())
    };
    let mut recreate_container = false;

    if container_exists(&client, CONTAINER_NAME).await? {
        let current_mounts = get_container_bind_mounts(&client, CONTAINER_NAME).await?;

        match resolve_mount_mismatch_action(&current_mounts, &bind_mounts, quiet) {
            MountMismatchAction::NoMismatch => {}
            MountMismatchAction::PromptRecreate => {
                display_mount_mismatch(&current_mounts, &bind_mounts);
                let confirm = dialoguer::Confirm::new()
                    .with_prompt("Recreate container with new mount configuration?")
                    .default(true)
                    .interact()?;
                if !confirm {
                    return Err(anyhow!(
                        "Container not recreated. Mount changes were not applied.\n\
                         To apply mount changes, run:\n  \
                         occ stop --remove\n  \
                         occ start"
                    ));
                }
                recreate_container = true;
            }
            MountMismatchAction::QuietError(message) => return Err(anyhow!(message)),
        }
    }

    // Create single spinner for the full operation
    let msg = crate::format_host_message(host_name.as_deref(), "Restarting service...");
    let spinner = CommandSpinner::new_maybe(&msg, quiet);

    if recreate_container {
        spinner.update(&crate::format_host_message(
            host_name.as_deref(),
            "Recreating container to apply mount changes...",
        ));
        if let Err(e) = stop_service(&client, true, None).await {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to recreate container",
            ));
            show_docker_error(&e);
            return Err(e.into());
        }
    } else if container_is_running(&client, CONTAINER_NAME).await? {
        spinner.update(&crate::format_host_message(
            host_name.as_deref(),
            "Stopping service...",
        ));
        if let Err(e) = stop_service(&client, false, None).await {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to stop",
            ));
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    // Start
    spinner.update(&crate::format_host_message(
        host_name.as_deref(),
        "Starting service...",
    ));
    match setup_and_start(
        &client,
        Some(port),
        None,
        Some(bind_addr),
        Some(config.cockpit_port),
        Some(config.cockpit_enabled && COCKPIT_EXPOSED),
        Some(systemd_enabled),
        bind_mounts_option,
    )
    .await
    {
        Ok(container_id) => {
            if let Err(e) = wait_for_service_ready(&client, bind_addr, port, &spinner).await {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Service failed to become ready",
                ));
                return Err(e);
            }

            if let Err(e) = wait_for_broker_ready(&client, &spinner).await {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Broker failed to become ready",
                ));
                return Err(e);
            }

            spinner.success(&crate::format_host_message(
                host_name.as_deref(),
                "Service restarted",
            ));

            if !quiet {
                let url = format!("http://{bind_addr}:{port}");
                println!();
                println!("URL:        {}", style(&url).cyan());
                println!(
                    "Container:  {}",
                    style(&container_id[..12.min(container_id.len())]).dim()
                );
            }
        }
        Err(e) => {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to start",
            ));
            show_docker_error(&e);
            return Err(e.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parsed_mount(host: &str, target: &str) -> ParsedMount {
        ParsedMount {
            host_path: PathBuf::from(host),
            container_path: target.to_string(),
            read_only: false,
        }
    }

    fn current_mount(source: &str, target: &str) -> ContainerBindMount {
        ContainerBindMount {
            source: source.to_string(),
            target: target.to_string(),
            read_only: false,
        }
    }

    #[test]
    fn resolve_mount_mismatch_action_mismatch_quiet_returns_error() {
        let current = vec![current_mount("/old", "/home/opencode/workspace")];
        let configured = vec![parsed_mount("/new", "/home/opencode/workspace")];
        let action = resolve_mount_mismatch_action(&current, &configured, true);
        assert!(matches!(action, MountMismatchAction::QuietError(_)));
    }

    #[test]
    fn resolve_mount_mismatch_action_mismatch_non_quiet_prompts_recreate() {
        let current = vec![current_mount("/old", "/home/opencode/workspace")];
        let configured = vec![parsed_mount("/new", "/home/opencode/workspace")];
        let action = resolve_mount_mismatch_action(&current, &configured, false);
        assert_eq!(action, MountMismatchAction::PromptRecreate);
    }

    #[test]
    fn resolve_mount_mismatch_action_no_mismatch() {
        let current = vec![current_mount("/same", "/home/opencode/workspace")];
        let configured = vec![parsed_mount("/same", "/home/opencode/workspace")];
        let action = resolve_mount_mismatch_action(&current, &configured, false);
        assert_eq!(action, MountMismatchAction::NoMismatch);
    }
}
