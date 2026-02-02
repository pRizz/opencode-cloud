//! Mount list subcommand

use anyhow::Result;
use clap::Args;
use comfy_table::{Cell, Table, presets::UTF8_FULL_CONDENSED};
use console::style;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::ParsedMount;
use std::path::Path;

#[derive(Args)]
pub struct MountListArgs {
    /// Output only host paths (for scripting)
    #[arg(long)]
    pub names_only: bool,

    /// Show resolved paths (as Docker sees them)
    #[arg(long, short)]
    pub resolved: bool,
}

/// Resolve a host path to what Docker will see
///
/// On macOS with Docker Desktop, paths are translated:
/// - /tmp -> /private/tmp -> /host_mnt/private/tmp
/// - /home/user -> /host_mnt/Users/user (if symlinked)
///
/// On Linux, paths are passed through unchanged.
fn resolve_docker_path(path: &Path) -> String {
    // Try to canonicalize (resolve symlinks)
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy();

    // On macOS, Docker Desktop mounts under /host_mnt
    if cfg!(target_os = "macos") {
        format!("/host_mnt{path_str}")
    } else {
        path_str.to_string()
    }
}

pub async fn cmd_mount_list(args: &MountListArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let config = load_config_or_default()?;

    if config.mounts.is_empty() {
        if !quiet && !args.names_only {
            println!("No mounts configured.");
            println!();
            println!(
                "Add a mount with: {}",
                style("occ mount add /host/path:/container/path").cyan()
            );
        }
        return Ok(());
    }

    // Names only mode for scripting
    if args.names_only {
        for mount_str in &config.mounts {
            if let Ok(parsed) = ParsedMount::parse(mount_str) {
                println!("{}", parsed.host_path.display());
            }
        }
        return Ok(());
    }

    // Table output
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);

    if args.resolved {
        table.set_header(vec![
            Cell::new("HOST PATH"),
            Cell::new("RESOLVED PATH"),
            Cell::new("CONTAINER PATH"),
            Cell::new("MODE"),
        ]);
    } else {
        table.set_header(vec![
            Cell::new("HOST PATH"),
            Cell::new("CONTAINER PATH"),
            Cell::new("MODE"),
        ]);
    }

    for mount_str in &config.mounts {
        match ParsedMount::parse(mount_str) {
            Ok(parsed) => {
                let mode = if parsed.read_only { "ro" } else { "rw" };
                if args.resolved {
                    let resolved = resolve_docker_path(&parsed.host_path);
                    table.add_row(vec![
                        Cell::new(parsed.host_path.display().to_string()),
                        Cell::new(resolved),
                        Cell::new(&parsed.container_path),
                        Cell::new(mode),
                    ]);
                } else {
                    table.add_row(vec![
                        Cell::new(parsed.host_path.display().to_string()),
                        Cell::new(&parsed.container_path),
                        Cell::new(mode),
                    ]);
                }
            }
            Err(_) => {
                // Show raw string for unparseable mounts
                if args.resolved {
                    table.add_row(vec![
                        Cell::new(mount_str),
                        Cell::new("-"),
                        Cell::new("(invalid)"),
                        Cell::new("-"),
                    ]);
                } else {
                    table.add_row(vec![
                        Cell::new(mount_str),
                        Cell::new("(invalid)"),
                        Cell::new("-"),
                    ]);
                }
            }
        }
    }

    println!("{table}");

    Ok(())
}
