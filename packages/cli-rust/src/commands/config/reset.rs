//! Config reset subcommand
//!
//! Resets configuration to default values.

use anyhow::Result;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::{Config, save_config};

/// Reset configuration to defaults
///
/// Prompts for confirmation unless --force is specified.
pub fn cmd_config_reset(force: bool, quiet: bool) -> Result<()> {
    // Prompt for confirmation unless forced
    if !force {
        let confirmed = Confirm::new()
            .with_prompt("Reset configuration to defaults? This cannot be undone.")
            .default(false)
            .interact()?;

        if !confirmed {
            if !quiet {
                println!("Reset cancelled.");
            }
            return Ok(());
        }
    }

    // Create default config and save
    let config = Config::default();
    save_config(&config)?;

    if !quiet {
        println!(
            "{} Configuration reset to defaults",
            style("Success:").green().bold()
        );
    }

    Ok(())
}
