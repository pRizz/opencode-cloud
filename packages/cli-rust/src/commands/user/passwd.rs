//! User passwd subcommand
//!
//! Changes a user's password.

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Password;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, set_user_password, user_exists};

/// Arguments for the user passwd command
#[derive(Args)]
pub struct UserPasswdArgs {
    /// Username to change password for
    pub username: String,
}

/// Change a user's password
pub async fn cmd_user_passwd(args: &UserPasswdArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let client = DockerClient::new()?;
    let username = &args.username;

    // Check if user exists
    if !user_exists(&client, CONTAINER_NAME, username).await? {
        bail!("User '{}' does not exist in the container", username);
    }

    // Prompt for new password
    let password = Password::new()
        .with_prompt("New password")
        .with_confirmation("Confirm new password", "Passwords do not match")
        .interact()?;

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    // Set the new password
    set_user_password(&client, CONTAINER_NAME, username, &password).await?;

    // Display success
    if !quiet {
        println!(
            "{} Password changed for user '{}'",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}
