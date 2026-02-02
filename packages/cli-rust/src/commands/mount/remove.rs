//! Mount remove subcommand

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::config::{load_config_or_default, save_config};
use opencode_cloud_core::docker::ParsedMount;

#[derive(Args)]
pub struct MountRemoveArgs {
    /// Host path of the mount to remove
    pub host_path: String,
}

pub async fn cmd_mount_remove(args: &MountRemoveArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let mut config = load_config_or_default()?;

    // Find and remove mount by host path
    let original_len = config.mounts.len();
    config.mounts.retain(|m| {
        ParsedMount::parse(m)
            .map(|p| p.host_path.to_string_lossy() != args.host_path)
            .unwrap_or(true) // Keep unparseable mounts
    });

    if config.mounts.len() == original_len {
        bail!(
            "No mount found for host path: {}\n\nList mounts with: occ mount list",
            args.host_path
        );
    }

    save_config(&config)?;

    if !quiet {
        println!("Removed mount: {}", style(&args.host_path).cyan());
        println!();
        println!(
            "{}",
            style("Note: Restart the container for changes to take effect.").dim()
        );
    }

    Ok(())
}
