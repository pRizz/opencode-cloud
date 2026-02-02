//! Cockpit command implementation
//!
//! Opens the Cockpit web console in the default browser.

use crate::constants::COCKPIT_EXPOSED;
use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::{CONTAINER_NAME, container_is_running};

/// Arguments for the cockpit command
#[derive(Args)]
pub struct CockpitArgs {}

/// Open Cockpit web console in browser
///
/// This command:
/// 1. Checks if Cockpit is enabled in config
/// 2. Checks if the container is running
/// 3. Opens the Cockpit URL in the default browser
pub async fn cmd_cockpit(_args: &CockpitArgs, maybe_host: Option<&str>, quiet: bool) -> Result<()> {
    if !COCKPIT_EXPOSED {
        if !quiet {
            println!(
                "{} This command is disabled in this release.",
                style("Note:").yellow()
            );
        }
        return Ok(());
    }
    // Load config
    let config = load_config_or_default()?;

    // Check if Cockpit is enabled
    if !config.cockpit_enabled {
        bail!(
            "{}\n\n\
             Cockpit is disabled in configuration.\n\n\
             {}: Cockpit requires Linux host with native Docker.\n\
             It does NOT work on macOS Docker Desktop.\n\n\
             To enable (Linux only):\n\
             1. {}\n\
             2. {}",
            style("Cockpit is disabled").yellow().bold(),
            style("Note").yellow(),
            style("occ config set cockpit_enabled true").cyan(),
            style("occ start --cached-rebuild").cyan()
        );
    }

    // Resolve Docker client (local or remote)
    let (client, _host_name) = crate::resolve_docker_client(maybe_host).await?;

    client
        .verify_connection()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let running = container_is_running(&client, CONTAINER_NAME).await?;
    if !running {
        // For 0.0.0.0 or :: bind addresses, use localhost for display
        let display_addr = if config.bind_address == "0.0.0.0" || config.bind_address == "::" {
            "127.0.0.1"
        } else {
            &config.bind_address
        };
        bail!(
            "{}\n\n\
             The container is not running. Cockpit runs inside the container.\n\n\
             Start the container: {}\n\
             Then access Cockpit:  {}",
            style("Container not running").yellow().bold(),
            style("occ start").cyan(),
            style(format!("http://{}:{}", display_addr, config.cockpit_port)).cyan()
        );
    }

    // Build URL
    // For 0.0.0.0 or :: bind addresses, use localhost for browser
    let browser_addr = if config.bind_address == "0.0.0.0" || config.bind_address == "::" {
        "127.0.0.1"
    } else {
        &config.bind_address
    };
    let url = format!("http://{}:{}", browser_addr, config.cockpit_port);

    if !quiet {
        println!("Opening Cockpit at: {}", style(&url).cyan());
        println!();
        println!(
            "{}: Log in with users created via '{}'",
            style("Tip").dim(),
            style("occ user add").cyan()
        );
    }

    // Open in browser
    if let Err(e) = webbrowser::open(&url) {
        if !quiet {
            eprintln!(
                "{} Failed to open browser: {}",
                style("Warning:").yellow(),
                e
            );
            eprintln!("Open manually: {}", style(&url).cyan());
        }
    }

    Ok(())
}
