//! Reset command implementation
//!
//! Provides destructive cleanup for containers, mounts, and host data.

use crate::commands::cleanup::{
    cleanup_mounts, collect_config_mounts, is_remote_host, load_config_for_mounts,
    remove_mounts_from_config,
};
use crate::commands::service::{StopSpinnerMessages, stop_service_with_spinner};
use crate::commands::start::{StartArgs, cmd_start};
use crate::output::{CommandSpinner, show_docker_error};
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand};
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::paths::{get_config_dir, get_data_dir};
use opencode_cloud_core::config::save_config;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DEFAULT_STOP_TIMEOUT_SECS, container_exists, remove_all_volumes,
};
use opencode_cloud_core::platform::{get_service_manager, is_service_registration_supported};
use std::fs;
use std::path::PathBuf;

/// Reset command arguments
#[derive(Args)]
pub struct ResetArgs {
    #[command(subcommand)]
    pub command: ResetCommands,
}

/// Reset command subcommands
#[derive(Subcommand)]
pub enum ResetCommands {
    /// Destroy the container and optionally remove volumes or mounts
    Container(ResetContainerArgs),
    /// Factory reset the host installation (container, volumes, mounts, config, data)
    Host(ResetHostArgs),
}

/// Arguments for reset container
#[derive(Args)]
pub struct ResetContainerArgs {
    /// Also remove Docker volumes (data deletion - requires --force)
    #[arg(long)]
    pub volumes: bool,

    /// Clean contents of configured bind mounts (requires --force)
    #[arg(long, conflicts_with = "purge_mounts")]
    pub clean_mounts: bool,

    /// Remove configured bind mount directories and config entries (requires --force)
    #[arg(long, conflicts_with = "clean_mounts")]
    pub purge_mounts: bool,

    /// Start the service after reset
    #[arg(long)]
    pub recreate: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    pub force: bool,
}

/// Arguments for reset host
#[derive(Args)]
pub struct ResetHostArgs {
    /// Skip confirmation prompts
    #[arg(long)]
    pub force: bool,
}

pub async fn cmd_reset(
    args: &ResetArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    match &args.command {
        ResetCommands::Container(container_args) => {
            cmd_reset_container(container_args, maybe_host, quiet, verbose).await
        }
        ResetCommands::Host(host_args) => {
            cmd_reset_host(host_args, maybe_host, quiet, verbose).await
        }
    }
}

async fn cmd_reset_container(
    args: &ResetContainerArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    let destructive = args.volumes || args.clean_mounts || args.purge_mounts;
    if destructive && !args.force {
        bail!(
            "Data-destructive flags require --force.\n\
             Use --force to confirm volume or mount deletion."
        );
    }

    if (args.clean_mounts || args.purge_mounts) && is_remote_host(maybe_host) {
        bail!(
            "Mount cleanup is only supported for local hosts.\n\
             Run without --host or use --host local on the machine where the mounts exist."
        );
    }

    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;
    client.verify_connection().await.map_err(|e| {
        let msg = crate::output::format_docker_error(&e);
        anyhow!("{msg}")
    })?;

    let mut errors = Vec::new();

    if container_exists(&client, CONTAINER_NAME).await? {
        let stop_result = stop_service_with_spinner(
            &client,
            host_name.as_deref(),
            quiet,
            true,
            DEFAULT_STOP_TIMEOUT_SECS,
            StopSpinnerMessages {
                action_message: "Stopping service...",
                update_label: "Stopping service",
                success_base_message: "Service stopped and removed",
                failure_message: "Failed to stop service",
            },
        )
        .await;
        if let Err(err) = stop_result {
            errors.push(format!("Failed to remove container: {err}"));
        }
    } else if !quiet {
        println!(
            "{}",
            style(crate::format_host_message(
                host_name.as_deref(),
                "Service container is already removed"
            ))
            .dim()
        );
    }

    if args.volumes {
        let spinner = CommandSpinner::new_maybe(
            &crate::format_host_message(host_name.as_deref(), "Removing Docker volumes..."),
            quiet,
        );
        match remove_all_volumes(&client).await {
            Ok(()) => spinner.success(&crate::format_host_message(
                host_name.as_deref(),
                "Docker volumes removed",
            )),
            Err(err) => {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Failed to remove Docker volumes",
                ));
                show_docker_error(&err);
                errors.push(format!("Failed to remove Docker volumes: {err}"));
            }
        }
    }

    if args.clean_mounts || args.purge_mounts {
        let (mut config, config_exists) = load_config_for_mounts(false)?;
        if config.mounts.is_empty() {
            if !quiet {
                println!("No mounts configured.");
            }
        } else {
            let collection = collect_config_mounts(&config);
            let result = cleanup_mounts(&collection.mounts, args.purge_mounts);

            if args.purge_mounts && config_exists {
                let purge_hosts: Vec<String> = collection
                    .mounts
                    .iter()
                    .map(|mount| mount.host_path.to_string_lossy().to_string())
                    .collect();
                let removed = remove_mounts_from_config(&mut config, &purge_hosts);
                if removed > 0 {
                    if let Err(err) = save_config(&config) {
                        errors.push(format!("Failed to update config mounts: {err}"));
                    }
                }
            }

            if !quiet {
                if args.purge_mounts {
                    if !result.purged.is_empty() {
                        println!("Purged mount directories:");
                        for path in &result.purged {
                            println!("  {}", style(path.display()).cyan());
                        }
                    }
                } else if !result.cleaned.is_empty() {
                    println!("Cleaned mount directories:");
                    for path in &result.cleaned {
                        println!("  {}", style(path.display()).cyan());
                    }
                }

                if !collection.skipped.is_empty() {
                    println!();
                    println!("{}", style("Skipped invalid mount entries:").yellow());
                    for item in &collection.skipped {
                        println!("  {}", style(item).yellow());
                    }
                }

                if !result.skipped.is_empty() {
                    println!();
                    println!("{}", style("Skipped mount paths:").yellow());
                    for item in &result.skipped {
                        println!("  {}", style(item).yellow());
                    }
                }
            }

            if result.has_errors() {
                for error in &result.errors {
                    errors.push(format!("Mount cleanup error: {error}"));
                }
            }
        }
    }

    if args.recreate {
        if errors.is_empty() {
            let start_args = StartArgs::default();
            if let Err(err) = cmd_start(&start_args, maybe_host, quiet, verbose).await {
                errors.push(format!("Failed to start service after reset: {err}"));
            }
        } else if !quiet {
            println!(
                "{}",
                style("Skipping recreate due to previous errors.").yellow()
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let mut message = String::from("Reset completed with errors:");
        for error in errors {
            message.push_str(&format!("\n  - {error}"));
        }
        Err(anyhow!(message))
    }
}

async fn cmd_reset_host(
    args: &ResetHostArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if is_remote_host(maybe_host) {
        bail!(
            "Host reset is only supported on the local machine.\n\
             Run without --host or use --host local."
        );
    }

    if !args.force {
        let confirmed = Confirm::new()
            .with_prompt(
                "This will remove all opencode-cloud data, config, mounts, and containers. Continue?",
            )
            .default(false)
            .interact()?;
        if !confirmed {
            if !quiet {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    let mut errors = Vec::new();
    let (config, _config_exists) = load_config_for_mounts(true)?;

    let docker_client = match crate::resolve_docker_client(maybe_host).await {
        Ok((client, host_name)) => {
            if let Err(err) = client.verify_connection().await {
                errors.push(format!("Docker unavailable: {err}"));
                None
            } else {
                Some((client, host_name))
            }
        }
        Err(err) => {
            errors.push(format!("Failed to connect to Docker: {err}"));
            None
        }
    };

    if let Some((client, host_name)) = docker_client.as_ref() {
        if container_exists(client, CONTAINER_NAME)
            .await
            .unwrap_or(false)
        {
            let stop_result = stop_service_with_spinner(
                client,
                host_name.as_deref(),
                quiet,
                true,
                DEFAULT_STOP_TIMEOUT_SECS,
                StopSpinnerMessages {
                    action_message: "Stopping service...",
                    update_label: "Stopping service",
                    success_base_message: "Service stopped and removed",
                    failure_message: "Failed to stop service",
                },
            )
            .await;
            if let Err(err) = stop_result {
                errors.push(format!("Failed to remove container: {err}"));
            }
        } else if !quiet {
            println!(
                "{}",
                style(crate::format_host_message(
                    host_name.as_deref(),
                    "Service container is already removed"
                ))
                .dim()
            );
        }

        let spinner = CommandSpinner::new_maybe(
            &crate::format_host_message(host_name.as_deref(), "Removing Docker volumes..."),
            quiet,
        );
        match remove_all_volumes(client).await {
            Ok(()) => spinner.success(&crate::format_host_message(
                host_name.as_deref(),
                "Docker volumes removed",
            )),
            Err(err) => {
                spinner.fail(&crate::format_host_message(
                    host_name.as_deref(),
                    "Failed to remove Docker volumes",
                ));
                show_docker_error(&err);
                errors.push(format!("Failed to remove Docker volumes: {err}"));
            }
        }
    }

    let collection = collect_config_mounts(&config);
    if !collection.mounts.is_empty() {
        let result = cleanup_mounts(&collection.mounts, true);

        if !quiet && !result.purged.is_empty() {
            println!("Purged mount directories:");
            for path in &result.purged {
                println!("  {}", style(path.display()).cyan());
            }
        }

        if !collection.skipped.is_empty() && !quiet {
            println!();
            println!("{}", style("Skipped invalid mount entries:").yellow());
            for item in &collection.skipped {
                println!("  {}", style(item).yellow());
            }
        }

        if !result.skipped.is_empty() && !quiet {
            println!();
            println!("{}", style("Skipped mount paths:").yellow());
            for item in &result.skipped {
                println!("  {}", style(item).yellow());
            }
        }

        if result.has_errors() {
            for error in &result.errors {
                errors.push(format!("Mount cleanup error: {error}"));
            }
        }
    }

    uninstall_service_registration(quiet, &mut errors);
    remove_dir_if_exists(get_config_dir(), "config", quiet, &mut errors);
    remove_dir_if_exists(get_data_dir(), "data", quiet, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        let mut message = String::from("Reset completed with errors:");
        for error in errors {
            message.push_str(&format!("\n  - {error}"));
        }
        Err(anyhow!(message))
    }
}

fn uninstall_service_registration(quiet: bool, errors: &mut Vec<String>) {
    if !is_service_registration_supported() {
        return;
    }

    let manager = match get_service_manager() {
        Ok(manager) => manager,
        Err(err) => {
            errors.push(format!("Failed to load service manager: {err}"));
            return;
        }
    };

    let installed = match manager.is_installed() {
        Ok(installed) => installed,
        Err(err) => {
            errors.push(format!("Failed to check service status: {err}"));
            return;
        }
    };

    if !installed {
        if !quiet {
            println!("{}", style("Service not installed.").dim());
        }
        return;
    }

    let spinner = CommandSpinner::new_maybe("Removing service registration...", quiet);
    match manager.uninstall() {
        Ok(()) => spinner.success("Service registration removed"),
        Err(err) => {
            spinner.fail("Failed to remove service registration");
            errors.push(format!("Failed to remove service registration: {err}"));
        }
    }
}

fn remove_dir_if_exists(path: Option<PathBuf>, label: &str, quiet: bool, errors: &mut Vec<String>) {
    let Some(path) = path else {
        return;
    };

    if !path.exists() {
        return;
    }

    if let Err(err) = fs::remove_dir_all(&path) {
        errors.push(format!(
            "Failed to remove {label} directory {}: {err}",
            path.display()
        ));
        return;
    }

    if !quiet {
        println!("Removed {label} directory: {}", style(path.display()).dim());
    }
}
