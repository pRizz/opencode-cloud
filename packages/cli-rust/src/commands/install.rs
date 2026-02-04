//! Install command implementation
//!
//! Registers the opencode-cloud service with the platform's service manager
//! (systemd on Linux, launchd on macOS) to start automatically on boot/login.

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::platform::{
    ServiceConfig, get_service_manager, is_service_registration_supported,
};

/// Arguments for the install command
#[derive(Args)]
pub struct InstallArgs {
    /// Skip confirmation prompt if service already installed
    #[arg(long)]
    force: bool,

    /// Show what would be done without making changes
    #[arg(long)]
    dry_run: bool,
}

/// Register the service with the platform's service manager
///
/// This command:
/// 1. Checks if the platform supports service registration
/// 2. Creates the service file (systemd unit or launchd plist)
/// 3. Registers and starts the service
///
/// The service will automatically restart on crash and start on boot/login
/// based on the configuration in config.json.
pub async fn cmd_install(args: &InstallArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // 1. Check platform support
    if !is_service_registration_supported() {
        return Err(anyhow!(
            "Service registration not supported on this platform.\n\
             Supported platforms: Linux (systemd), macOS (launchd)"
        ));
    }

    // 2. Load config first to get boot_mode
    let config = load_config_or_default()?;

    // 3. Get service manager with correct boot_mode
    let manager = get_service_manager(&config.boot_mode)?;

    // 3. Check if already installed
    if manager.is_installed()? {
        if args.dry_run {
            println!(
                "Would reinstall service at: {}",
                manager.service_file_path().display()
            );
            return Ok(());
        }

        if !args.force {
            // Use dialoguer for confirmation
            let confirm = Confirm::new()
                .with_prompt("Service already installed. Reinstall?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("Aborted.");
                return Ok(());
            }

            // Uninstall first
            let spinner = CommandSpinner::new_maybe("Removing existing service...", quiet);
            manager.uninstall()?;
            spinner.success("Existing service removed");
        } else {
            // Force mode - uninstall silently
            manager.uninstall()?;
        }
    } else if args.dry_run {
        println!(
            "Would install service at: {}",
            manager.service_file_path().display()
        );
        return Ok(());
    }

    // 4. Show spinner during install
    let spinner = CommandSpinner::new_maybe("Installing service...", quiet);

    // 5. Get executable path (current binary)
    let executable_path = std::env::current_exe()?;

    // 6. Build ServiceConfig (config already loaded above for boot_mode)
    let service_config = ServiceConfig {
        executable_path,
        restart_retries: config.restart_retries,
        restart_delay: config.restart_delay,
        boot_mode: config.boot_mode.clone(),
    };

    // 8. Perform install
    let result = manager.install(&service_config)?;

    spinner.success("Service installed");

    // 9. Print success details
    if !quiet {
        println!();
        println!(
            "Service file: {}",
            style(result.service_file_path.display()).dim()
        );
        println!("Service name: {}", result.service_name);
        if result.started {
            println!("Status:       {}", style("running").green());
        }
        println!();
        let boot_desc = if config.boot_mode == "system" {
            "boot"
        } else {
            "login"
        };
        println!("The service will start automatically on {boot_desc}.");
    }

    Ok(())
}
