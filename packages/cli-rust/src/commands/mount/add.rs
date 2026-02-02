//! Mount add subcommand

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::config::{load_config_or_default, save_config};
use opencode_cloud_core::docker::{ParsedMount, check_container_path_warning, validate_mount_path};

#[derive(Args)]
pub struct MountAddArgs {
    /// Mount specification: /host/path:/container/path[:ro]
    pub mount_spec: String,

    /// Skip path validation (useful for paths that will exist later)
    #[arg(long)]
    pub no_validate: bool,

    /// Force add even if warning about system paths
    #[arg(long, short)]
    pub force: bool,
}

pub async fn cmd_mount_add(args: &MountAddArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Parse the mount spec
    let parsed = ParsedMount::parse(&args.mount_spec)?;

    // Validate host path unless --no-validate
    if !args.no_validate {
        validate_mount_path(&parsed.host_path)?;
    }

    // Check for system path warning
    if let Some(warning) = check_container_path_warning(&parsed.container_path) {
        if !args.force {
            eprintln!("{}", style(&warning).yellow());
            eprintln!();
            eprintln!("Use {} to add anyway.", style("--force").cyan());
            bail!("Mount target is a system path. Use --force to override.");
        }
        if !quiet {
            eprintln!("{}", style(&warning).yellow());
        }
    }

    // Load config and add mount
    let mut config = load_config_or_default()?;

    // Check for duplicate (by host path)
    let host_str = parsed.host_path.to_string_lossy();
    let already_exists = config.mounts.iter().any(|m| {
        ParsedMount::parse(m)
            .map(|p| p.host_path.to_string_lossy() == host_str)
            .unwrap_or(false)
    });

    if already_exists {
        if !quiet {
            println!(
                "Mount for {} already configured. Remove first with: occ mount remove {}",
                style(&host_str).cyan(),
                host_str
            );
        }
        return Ok(());
    }

    config.mounts.push(args.mount_spec.clone());
    save_config(&config)?;

    if !quiet {
        let mode = if parsed.read_only { "ro" } else { "rw" };
        println!(
            "Added mount: {} -> {} ({})",
            style(&host_str).cyan(),
            style(&parsed.container_path).cyan(),
            mode
        );
        println!();
        println!(
            "{}",
            style("Note: Restart the container for changes to take effect.").dim()
        );
    }

    Ok(())
}
