//! Config subcommand implementations
//!
//! Provides `occ config` subcommands for viewing and managing configuration.

mod get;
mod reset;
mod show;

use anyhow::Result;
use clap::Subcommand;
use opencode_cloud_core::Config;

pub use get::cmd_config_get;
pub use reset::cmd_config_reset;
pub use show::cmd_config_show;

/// Configuration management subcommands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration (default when no subcommand given)
    Show {
        /// Output as JSON instead of table format
        #[arg(long)]
        json: bool,
    },
    /// Get a single configuration value
    Get {
        /// Configuration key (e.g., "port", "auth_username", "bind")
        key: String,
    },
    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
}

/// Handle config subcommands
///
/// Routes to the appropriate handler based on the subcommand.
/// If no subcommand is given, defaults to Show (handled by clap default_subcommand).
pub fn cmd_config(cmd: ConfigCommands, config: &Config, quiet: bool) -> Result<()> {
    match cmd {
        ConfigCommands::Show { json } => cmd_config_show(config, json, quiet),
        ConfigCommands::Get { key } => cmd_config_get(config, &key, quiet),
        ConfigCommands::Reset { force } => cmd_config_reset(force, quiet),
    }
}
