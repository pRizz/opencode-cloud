//! occ host remove - Remove a remote host

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::{load_hosts, save_hosts};

/// Arguments for host remove command
#[derive(Args)]
pub struct HostRemoveArgs {
    /// Name of the host to remove
    pub name: String,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub force: bool,
}

pub async fn cmd_host_remove(args: &HostRemoveArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Load existing hosts
    let mut hosts = load_hosts()?;

    // Check if host exists
    if !hosts.has_host(&args.name) {
        bail!("Host '{}' not found.", args.name);
    }

    // Confirm unless --force
    if !args.force && !quiet {
        let is_default = hosts.default_host.as_deref() == Some(&args.name);
        let prompt = if is_default {
            format!(
                "Remove host '{}' (currently the default)?",
                style(&args.name).cyan()
            )
        } else {
            format!("Remove host '{}'?", style(&args.name).cyan())
        };

        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Remove host
    hosts.remove_host(&args.name);

    // Save
    save_hosts(&hosts)?;

    if !quiet {
        println!(
            "{} Host '{}' removed.",
            style("Removed:").yellow(),
            style(&args.name).cyan()
        );
    }

    Ok(())
}
