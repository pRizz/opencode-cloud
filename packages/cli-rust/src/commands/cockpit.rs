//! Cockpit command implementation
//!
//! Opens the Cockpit web console in the default browser.

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::config::load_config;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, container_is_running};

/// Arguments for the cockpit command
#[derive(Args)]
pub struct CockpitArgs {}

/// Open Cockpit web console in browser
///
/// This command:
/// 1. Checks if Cockpit is enabled in config
/// 2. Checks if the container is running
/// 3. Opens the Cockpit URL in the default browser
pub async fn cmd_cockpit(_args: &CockpitArgs, quiet: bool) -> Result<()> {
    // Load config
    let config = load_config()?;

    // Check if Cockpit is enabled
    if !config.cockpit_enabled {
        bail!(
            "{}\n\n\
             Cockpit is disabled in configuration.\n\n\
             Enable it with: {}\n\
             Then rebuild:   {}",
            style("Cockpit is disabled").yellow().bold(),
            style("occ config set cockpit_enabled true").cyan(),
            style("occ start --cached-rebuild").cyan()
        );
    }

    // Connect to Docker and check container status
    let client = DockerClient::new().map_err(|e| anyhow::anyhow!("{}", e))?;
    client
        .verify_connection()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

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
