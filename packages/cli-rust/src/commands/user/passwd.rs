//! User passwd subcommand
//!
//! Changes a user's password.

use crate::passwords::{generate_random_password, print_generated_password};
use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Password;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, persist_user, set_user_password, user_exists,
};

/// Arguments for the user passwd command
#[derive(Args)]
#[command(
    after_help = "Tip: Use --generate (-g) to auto-generate a secure password instead of typing one."
)]
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
        if !quiet {
            println!(
                "  {} Use {} to auto-generate a secure password.",
                style("Tip:").cyan(),
                style("--generate (-g)").bold()
            );
        }
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

    // Persist updated credentials for rebuild/update restores
    persist_user(client, CONTAINER_NAME, username).await?;

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

        if args.generate {
            print_generated_password(
                &password,
                "Save this password securely - it won't be shown again.",
            );
        }
    }

    Ok(())
}
