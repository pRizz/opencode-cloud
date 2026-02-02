//! User remove subcommand
//!
//! Removes a user from the container.

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, delete_user, list_users, remove_persisted_user, user_exists,
};
use opencode_cloud_core::{load_config_or_default, save_config};

/// Arguments for the user remove command
#[derive(Args)]
pub struct UserRemoveArgs {
    /// Username to remove
    pub username: String,

    /// Skip confirmation prompt
    #[arg(long, short)]
    pub force: bool,
}

/// System user that cannot be removed (container depends on it)
const PROTECTED_USER: &str = "opencode";

/// Remove a user from the container
pub async fn cmd_user_remove(
    client: &DockerClient,
    args: &UserRemoveArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let username = &args.username;

    // Protect the opencode system user - cannot be removed even with --force
    if username == PROTECTED_USER {
        bail!(
            "Cannot remove '{PROTECTED_USER}' - this is a protected system user required for the container to function.\n\n\
            To manage authentication users, use:\n  \
            occ user add <username>\n  \
            occ user remove <username>"
        );
    }

    // Check if user exists
    if !user_exists(client, CONTAINER_NAME, username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    // Load config for later update
    let mut config = load_config_or_default()?;

    // Check if this is the last user in the container (not just tracked users)
    let container_users = list_users(client, CONTAINER_NAME).await?;
    let is_last_user = container_users.len() == 1;
    if is_last_user && !args.force {
        bail!(
            "Cannot remove last user. Add another user first or use --force.\n\n\
            To add a new user:\n  \
            occ user add <username>\n\n\
            To force removal:\n  \
            occ user remove {username} --force"
        );
    }

    // Confirm removal
    if !args.force {
        let confirm = Confirm::new()
            .with_prompt(format!("Remove user '{username}'?"))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirm {
            if !quiet {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    // Delete the user
    delete_user(client, CONTAINER_NAME, username).await?;

    // Remove persisted record if present
    remove_persisted_user(client, CONTAINER_NAME, username).await?;

    // Update config - remove username from users array
    config.users.retain(|u| u != username);
    save_config(&config)?;

    // Display success
    if !quiet {
        println!(
            "{} User '{}' removed successfully",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}
