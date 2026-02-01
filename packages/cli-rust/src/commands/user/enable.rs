//! User enable/disable subcommands
//!
//! Enables or disables user accounts.

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, lock_user, persist_user, unlock_user, user_exists,
};

/// Arguments for the user enable command
#[derive(Args)]
pub struct UserEnableArgs {
    /// Username to enable
    pub username: String,
}

/// Arguments for the user disable command
#[derive(Args)]
pub struct UserDisableArgs {
    /// Username to disable
    pub username: String,
}

/// Enable a user account
pub async fn cmd_user_enable(
    client: &DockerClient,
    args: &UserEnableArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let username = &args.username;

    // Check if user exists
    if !user_exists(client, CONTAINER_NAME, username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    // Unlock the user account
    unlock_user(client, CONTAINER_NAME, username).await?;

    // Persist updated lock state for rebuild/update restores
    persist_user(client, CONTAINER_NAME, username).await?;

    // Display success
    if !quiet {
        println!(
            "{} User '{}' enabled",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}

/// Disable a user account
pub async fn cmd_user_disable(
    client: &DockerClient,
    args: &UserDisableArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let username = &args.username;

    // Check if user exists
    if !user_exists(client, CONTAINER_NAME, username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    // Lock the user account
    lock_user(client, CONTAINER_NAME, username).await?;

    // Persist updated lock state for rebuild/update restores
    persist_user(client, CONTAINER_NAME, username).await?;

    // Display success
    if !quiet {
        println!(
            "{} User '{}' disabled",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}
