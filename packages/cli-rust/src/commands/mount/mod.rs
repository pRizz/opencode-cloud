//! Mount management subcommand implementations
//!
//! Provides `occ mount` subcommands for managing bind mounts.

mod add;
mod clean;
mod list;
mod remove;

use anyhow::Result;
use clap::{Args, Subcommand};

pub use add::cmd_mount_add;
pub use clean::cmd_mount_clean;
pub use list::cmd_mount_list;
pub use remove::cmd_mount_remove;

/// Mount management command arguments
#[derive(Args)]
pub struct MountArgs {
    #[command(subcommand)]
    pub command: MountCommands,
}

/// Mount management subcommands
#[derive(Subcommand)]
pub enum MountCommands {
    /// Add a bind mount to configuration
    Add(add::MountAddArgs),
    /// Remove a bind mount from configuration
    Remove(remove::MountRemoveArgs),
    /// List configured bind mounts
    List(list::MountListArgs),
    /// Clean contents of bind mounts
    Clean(clean::MountCleanArgs),
}

/// Handle mount command
pub async fn cmd_mount(
    args: &MountArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    match &args.command {
        MountCommands::Add(add_args) => cmd_mount_add(add_args, quiet, verbose).await,
        MountCommands::Remove(remove_args) => cmd_mount_remove(remove_args, quiet, verbose).await,
        MountCommands::List(list_args) => cmd_mount_list(list_args, quiet, verbose).await,
        MountCommands::Clean(clean_args) => {
            cmd_mount_clean(clean_args, maybe_host, quiet, verbose).await
        }
    }
}
