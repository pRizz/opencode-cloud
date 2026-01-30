//! Config subcommand implementations
//!
//! Provides `occ config` subcommands for viewing and managing configuration.

mod env;
mod get;
mod reset;
mod set;
mod show;

use anyhow::Result;
use clap::{Args, Subcommand};
use opencode_cloud_core::Config;

pub use env::{EnvCommands, cmd_config_env};
pub use get::cmd_config_get;
pub use reset::cmd_config_reset;
pub use set::cmd_config_set;
pub use show::cmd_config_show;

/// Configuration command arguments
#[derive(Args)]
pub struct ConfigArgs {
    /// Output as JSON instead of table format
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Option<ConfigSubcommands>,
}

/// Configuration management subcommands
#[derive(Subcommand)]
pub enum ConfigSubcommands {
    /// Show current configuration
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
    /// Set a configuration value
    Set {
        /// Configuration key to set (e.g., "port", "username", "password")
        key: String,
        /// Value to set (omit for password to prompt securely)
        value: Option<String>,
        /// Skip confirmation prompts (use with care)
        #[arg(long)]
        force: bool,
    },
    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
    /// Manage container environment variables
    #[command(subcommand)]
    Env(EnvCommands),
}

/// Handle config command
///
/// Routes to the appropriate handler based on the subcommand.
/// If no subcommand is given, defaults to Show.
pub fn cmd_config(args: ConfigArgs, config: &Config, quiet: bool) -> Result<()> {
    match args.command {
        Some(ConfigSubcommands::Show { json }) => cmd_config_show(config, json, quiet),
        Some(ConfigSubcommands::Get { key }) => cmd_config_get(config, &key, quiet),
        Some(ConfigSubcommands::Set { key, value, force }) => {
            cmd_config_set(&key, value.as_deref(), quiet, force)
        }
        Some(ConfigSubcommands::Reset { force }) => cmd_config_reset(force, quiet),
        Some(ConfigSubcommands::Env(env_cmd)) => cmd_config_env(env_cmd, quiet),
        None => {
            // Default to show when no subcommand given
            cmd_config_show(config, args.json, quiet)
        }
    }
}
