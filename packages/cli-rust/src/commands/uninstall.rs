//! Uninstall command implementation
//!
//! Removes the opencode-cloud service registration from the platform's
//! service manager (systemd on Linux, launchd on macOS).

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::paths::{get_config_dir, get_data_dir};
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, DockerError, container_is_running, remove_all_volumes,
    stop_container,
};
use opencode_cloud_core::platform::{get_service_manager, is_service_registration_supported};

/// Arguments for the uninstall command
#[derive(Args)]
pub struct UninstallArgs {
    /// Also remove Docker volumes (data deletion - requires --force)
    #[arg(long)]
    volumes: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    force: bool,
}

/// Remove the service registration from the platform's service manager
///
/// This command:
/// 1. Stops the container if running
/// 2. Removes the service registration (systemd unit or launchd plist)
/// 3. Optionally removes Docker volumes (with --volumes --force)
///
/// The command is idempotent - exits 0 if service is not installed.
pub async fn cmd_uninstall(args: &UninstallArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // 1. Validate --volumes requires --force
    if args.volumes && !args.force {
        return Err(anyhow!(
            "The --volumes flag requires --force to confirm data deletion.\n\
             Run: occ uninstall --volumes --force"
        ));
    }

    // 2. Check platform support
    if !is_service_registration_supported() {
        return Err(anyhow!(
            "Service registration not supported on this platform.\n\
             Supported platforms: Linux (systemd), macOS (launchd)"
        ));
    }

    // 3. Get service manager
    let manager = get_service_manager()?;

    // 4. Check if installed
    if !manager.is_installed()? {
        if !quiet {
            println!("{}", style("Service not installed.").dim());
        }
        return Ok(()); // Exit 0 - idempotent
    }

    // 5. Confirm uninstallation (unless --force)
    if !args.force {
        let confirm = Confirm::new()
            .with_prompt("This will remove the service registration. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirm {
            if !quiet {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    // 6. Stop container if running (using existing stop logic)
    let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
    // Try to stop - ignore errors if not running
    let _ = stop_container_if_running().await;
    spinner.success("Service stopped");

    // 7. Uninstall service registration
    let spinner = CommandSpinner::new_maybe("Removing service registration...", quiet);
    let service_file = manager.service_file_path();
    manager.uninstall()?;
    spinner.success("Service registration removed");

    // 8. Optionally remove volumes
    if args.volumes {
        let spinner = CommandSpinner::new_maybe("Removing Docker volumes...", quiet);
        remove_volumes().await?;
        spinner.success("Docker volumes removed");
    }

    // 9. Print what was removed
    if !quiet {
        println!();
        println!("Removed: {}", style(service_file.display()).dim());
        if args.volumes {
            println!("Removed: Docker volumes (all data deleted)");
        }
        println!();
        println!("Service will no longer start automatically.");

        // 10. Show remaining files for manual cleanup
        let config_dir = get_config_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/opencode-cloud".to_string());
        let data_dir = get_data_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.local/share/opencode-cloud".to_string());

        println!();
        println!("Files retained (for reinstall):");
        println!("  Config: {}", style(&config_dir).dim());
        println!("  Data:   {}", style(&data_dir).dim());
        println!();
        println!("To completely remove all files:");
        println!("  rm -rf {} {}", config_dir, data_dir);
    }

    Ok(())
}

/// Stop container if running (helper)
async fn stop_container_if_running() -> Result<()> {
    // Similar to cmd_stop but ignores "not running" state
    let client = match DockerClient::new() {
        Ok(c) => c,
        Err(_) => return Ok(()), // Docker not available - nothing to stop
    };

    if client.verify_connection().await.is_err() {
        return Ok(()); // Docker not running - nothing to stop
    }

    // Check if container is running
    match container_is_running(&client, CONTAINER_NAME).await {
        Ok(true) => {
            // Try to stop - ignore 404/not running errors
            match stop_container(&client, CONTAINER_NAME, Some(30)).await {
                Ok(()) => Ok(()),
                Err(DockerError::Container(msg)) if msg.contains("is not running") => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        Ok(false) => Ok(()), // Not running
        Err(_) => Ok(()),    // Container doesn't exist
    }
}

/// Remove Docker volumes (helper)
async fn remove_volumes() -> Result<()> {
    let client = DockerClient::new()?;
    client.verify_connection().await?;
    remove_all_volumes(&client).await?;
    Ok(())
}
