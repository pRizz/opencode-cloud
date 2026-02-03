//! Mount clean subcommand

use crate::commands::cleanup::{
    cleanup_mounts, collect_config_mounts, is_remote_host, load_config_for_mounts,
    remove_mounts_from_config,
};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::save_config;

#[derive(Args)]
pub struct MountCleanArgs {
    /// Remove mount directories and remove config entries
    #[arg(long)]
    pub purge: bool,

    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

pub async fn cmd_mount_clean(
    args: &MountCleanArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if is_remote_host(maybe_host) {
        return Err(anyhow!(
            "Mount cleanup is only supported for local hosts.\n\
             Run without --remote-host or use --local on the machine where the mounts exist."
        ));
    }

    if !args.force {
        let prompt = if args.purge {
            "This will delete all contents of configured bind mounts and remove the mount directories. Continue?"
        } else {
            "This will delete all contents of configured bind mounts. Continue?"
        };
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            if !quiet {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    let (mut config, config_exists) = load_config_for_mounts(false)?;
    if !config_exists || config.mounts.is_empty() {
        if !quiet {
            println!("No mounts configured.");
        }
        return Ok(());
    }

    let collection = collect_config_mounts(&config);
    if collection.mounts.is_empty() {
        if !quiet {
            println!("No valid mounts found to clean.");
            if !collection.skipped.is_empty() {
                println!();
                println!("{}", style("Skipped invalid mount entries:").yellow());
                for item in &collection.skipped {
                    println!("  {}", style(item).yellow());
                }
            }
        }
        return Ok(());
    }

    let result = cleanup_mounts(&collection.mounts, args.purge);

    if args.purge {
        let purge_hosts: Vec<String> = collection
            .mounts
            .iter()
            .map(|mount| mount.host_path.to_string_lossy().to_string())
            .collect();
        let removed = remove_mounts_from_config(&mut config, &purge_hosts);
        if removed > 0 {
            save_config(&config)?;
        }
    }

    if !quiet {
        if args.purge {
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
        let mut message = String::from("Mount cleanup completed with errors:");
        for error in &result.errors {
            message.push_str(&format!("\n  - {error}"));
        }
        return Err(anyhow!(message));
    }

    Ok(())
}
