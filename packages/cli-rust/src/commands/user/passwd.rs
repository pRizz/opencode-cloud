//! User passwd subcommand
//!
//! Changes a user's password.

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Password;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, set_user_password, user_exists};
use rand::Rng;
use rand::distr::Alphanumeric;

use crate::constants::password_length;

/// Arguments for the user passwd command
#[derive(Args)]
pub struct UserPasswdArgs {
    /// Username to change password for
    pub username: String,

    /// Generate a random secure password instead of prompting
    #[arg(long, short)]
    pub generate: bool,

    /// Print only the generated password for scripting
    #[arg(long)]
    pub print_password_only: bool,
}

fn generate_random_password() -> String {
    // ThreadRng is a CSPRNG seeded from the OS.
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(password_length())
        .map(char::from)
        .collect()
}

/// Change a user's password
pub async fn cmd_user_passwd(
    client: &DockerClient,
    args: &UserPasswdArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if args.print_password_only && !args.generate {
        bail!("--print-password-only requires --generate");
    }

    let username = &args.username;

    // Check if user exists
    if !user_exists(client, CONTAINER_NAME, username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    // Prompt for new password (or generate)
    let password = if args.generate {
        generate_random_password()
    } else {
        Password::new()
            .with_prompt("New password")
            .with_confirmation("Confirm new password", "Passwords do not match")
            .interact()?
    };

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    // Set the new password
    set_user_password(client, CONTAINER_NAME, username, &password).await?;

    if args.print_password_only {
        println!("{password}");
        return Ok(());
    }

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
