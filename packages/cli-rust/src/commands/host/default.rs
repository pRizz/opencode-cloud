//! occ host default - Set or show the default host

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::{load_hosts, save_hosts};

/// Arguments for host default command
#[derive(Args)]
pub struct HostDefaultArgs {
    /// Name of the host to set as default (omit to show current, "local" to clear)
    pub name: Option<String>,
}

pub async fn cmd_host_default(args: &HostDefaultArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let mut hosts = load_hosts()?;

    match &args.name {
        None => {
            // Show current default
            match &hosts.default_host {
                Some(name) => {
                    if quiet {
                        println!("{name}");
                    } else {
                        println!("Default host: {}", style(name).cyan());
                    }
                }
                None => {
                    if quiet {
                        println!("local");
                    } else {
                        println!("Default host: {} (local Docker)", style("local").cyan());
                    }
                }
            }
            Ok(())
        }
        Some(name) if name == "local" || name == "none" || name.is_empty() => {
            // Clear default
            if hosts.default_host.is_none() {
                if !quiet {
                    println!("Default is already local Docker.");
                }
                return Ok(());
            }

            hosts.set_default(None);
            save_hosts(&hosts)?;

            if !quiet {
                println!(
                    "{} Default cleared. Commands will use local Docker.",
                    style("Updated:").green()
                );
            }
            Ok(())
        }
        Some(name) => {
            // Set default
            if !hosts.has_host(name) {
                bail!("Host '{name}' not found. Add it first with: occ host add {name} <hostname>");
            }

            hosts.set_default(Some(name.clone()));
            save_hosts(&hosts)?;

            if !quiet {
                println!(
                    "{} Default host set to '{}'.",
                    style("Updated:").green(),
                    style(name).cyan()
                );
                println!(
                    "  {} Commands will now target {} unless {} is specified.",
                    style("Note:").dim(),
                    style(name).cyan(),
                    style("--remote-host or --local").yellow()
                );
            }
            Ok(())
        }
    }
}
