//! User management subcommand implementations
//!
//! Provides `occ user` subcommands for managing container users.

mod add;
mod enable;
mod list;
mod passwd;
mod remove;

use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use opencode_cloud_core::docker::{CONTAINER_NAME, container_is_running};

pub use add::cmd_user_add;
pub use enable::{cmd_user_disable, cmd_user_enable};
pub use list::cmd_user_list;
pub use passwd::cmd_user_passwd;
pub use remove::cmd_user_remove;

/// User management command arguments
#[derive(Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommands,
}

/// User management subcommands
#[derive(Subcommand)]
pub enum UserCommands {
    /// Add a new user to the container
    Add(add::UserAddArgs),
    /// Remove a user from the container
    Remove(remove::UserRemoveArgs),
    /// List users in the container
    List(list::UserListArgs),
    /// Change a user's password
    Passwd(passwd::UserPasswdArgs),
    /// Enable a user account
    Enable(enable::UserEnableArgs),
    /// Disable a user account
    Disable(enable::UserDisableArgs),
}

/// Handle user command
///
/// Routes to the appropriate handler based on the subcommand.
pub async fn cmd_user(
    args: &UserArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    // Check container is running first
    if !container_is_running(&client, CONTAINER_NAME).await? {
        let msg = if let Some(name) = &host_name {
            format!("Container not running on {name}. Start with `occ start --host {name}`.")
        } else {
            "Container not running. Start with `occ start` first.".to_string()
        };
        bail!(msg);
    }

    match &args.command {
        UserCommands::Add(add_args) => cmd_user_add(&client, add_args, quiet, verbose).await,
        UserCommands::Remove(remove_args) => {
            cmd_user_remove(&client, remove_args, quiet, verbose).await
        }
        UserCommands::List(list_args) => cmd_user_list(&client, list_args, quiet, verbose).await,
        UserCommands::Passwd(passwd_args) => {
            cmd_user_passwd(&client, passwd_args, quiet, verbose).await
        }
        UserCommands::Enable(enable_args) => {
            cmd_user_enable(&client, enable_args, quiet, verbose).await
        }
        UserCommands::Disable(disable_args) => {
            cmd_user_disable(&client, disable_args, quiet, verbose).await
        }
    }
}
