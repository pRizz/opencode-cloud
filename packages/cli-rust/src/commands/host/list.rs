//! occ host list - List all configured hosts

use anyhow::Result;
use clap::Args;
use comfy_table::{Cell, Color, Table};
use console::style;
use opencode_cloud_core::load_hosts;

/// Arguments for host list command
#[derive(Args)]
pub struct HostListArgs {
    /// Filter by group
    #[arg(short, long)]
    pub group: Option<String>,

    /// Show only host names (for scripting)
    #[arg(long)]
    pub names_only: bool,
}

pub async fn cmd_host_list(args: &HostListArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let hosts = load_hosts()?;

    if hosts.hosts.is_empty() {
        if !quiet && !args.names_only {
            println!("No hosts configured.");
            println!();
            println!(
                "  {} {}",
                style("Add one with:").dim(),
                style("occ host add <name> <hostname>").yellow()
            );
        }
        return Ok(());
    }

    // Filter by group if specified
    let filtered: Vec<_> = hosts
        .hosts
        .iter()
        .filter(|(_, config)| {
            args.group
                .as_ref()
                .map(|g| config.groups.contains(g))
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        if !quiet && !args.names_only {
            println!(
                "No hosts found in group '{}'.",
                args.group.as_deref().unwrap_or("")
            );
        }
        return Ok(());
    }

    // Names only mode (for scripting)
    if args.names_only || quiet {
        for (name, _) in &filtered {
            println!("{}", name);
        }
        return Ok(());
    }

    // Build table
    let mut table = Table::new();
    table.set_header(vec![
        "Name", "Hostname", "User", "Port", "Groups", "Default",
    ]);

    for (name, config) in filtered {
        let is_default = hosts.default_host.as_deref() == Some(name.as_str());

        let name_cell = if is_default {
            Cell::new(name).fg(Color::Cyan)
        } else {
            Cell::new(name)
        };

        let port_str = config
            .port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "22".to_string());
        let groups_str = if config.groups.is_empty() {
            "-".to_string()
        } else {
            config.groups.join(", ")
        };
        let default_str = if is_default { "*" } else { "" };

        table.add_row(vec![
            name_cell,
            Cell::new(&config.hostname),
            Cell::new(&config.user),
            Cell::new(port_str),
            Cell::new(groups_str),
            Cell::new(default_str),
        ]);
    }

    println!("{table}");

    if let Some(default) = &hosts.default_host {
        println!();
        println!(
            "  {} {}",
            style("Default host:").dim(),
            style(default).cyan()
        );
    }

    Ok(())
}
