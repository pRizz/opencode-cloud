//! User add subcommand
//!
//! Creates a new user in the container with a password.

use crate::passwords::{generate_random_password, print_generated_password};
use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::{Input, Password};
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, create_user, persist_user, set_user_password, user_exists,
};
use opencode_cloud_core::{load_config_or_default, save_config};

/// Arguments for the user add command
#[derive(Args)]
#[command(
    after_help = "Tip: Press Enter at the password prompt to auto-generate and display a secure password, or use --generate (-g) for non-interactive use."
)]
pub struct UserAddArgs {
    /// Username to create (default: opencode if not provided)
    pub username: Option<String>,

    /// Generate a random secure password instead of prompting
    #[arg(long, short)]
    pub generate: bool,

    /// Print only the generated password for scripting
    #[arg(long)]
    pub print_password_only: bool,
}

/// Validate username according to rules
/// - Non-empty
/// - 3-32 characters
/// - Alphanumeric + underscore only
fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if username.len() < 3 {
        return Err("Username must be at least 3 characters".to_string());
    }
    if username.len() > 32 {
        return Err("Username must be at most 32 characters".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username must contain only letters, numbers, and underscores".to_string());
    }
    Ok(())
}

/// Add a new user to the container
pub async fn cmd_user_add(
    client: &DockerClient,
    args: &UserAddArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if args.print_password_only && !args.generate {
        bail!("--print-password-only requires --generate");
    }

    // Get username - prompt if not provided
    let username = if let Some(ref name) = args.username {
        validate_username(name).map_err(|e| anyhow::anyhow!("{e}"))?;
        name.clone()
    } else {
        Input::new()
            .with_prompt("Username")
            .default("opencode".to_string())
            .validate_with(|input: &String| validate_username(input))
            .interact_text()?
    };

    // Check if user already exists
    if user_exists(client, CONTAINER_NAME, &username).await? {
        bail!("User '{username}' already exists in the container");
    }

    // Get password
    let mut generated = args.generate;
    let password = if args.generate {
        generate_random_password()
    } else {
        // Explain what password is being requested to avoid confusion with sudo
        if !quiet {
            println!();
            println!(
                "{}",
                style("Set a password for the new container user.").dim()
            );
            println!(
                "{}",
                style("This will be used for opencode web login.").dim()
            );
            println!(
                "{}",
                style(
                    "Authentication is handled by the system via PAM - we don't store passwords."
                )
                .dim()
            );
            println!(
                "  {} Use {} to auto-generate a secure password.",
                style("Tip:").cyan(),
                style("--generate (-g)").bold()
            );
            println!(
                "  {} Press Enter to auto-generate and display a secure password.",
                style("Tip:").cyan()
            );
        }
        loop {
            let pwd = Password::new()
                .with_prompt("Password")
                .allow_empty_password(true)
                .interact()?;

            if pwd.is_empty() {
                generated = true;
                break generate_random_password();
            }

            let confirm = Password::new().with_prompt("Confirm password").interact()?;
            if pwd != confirm {
                eprintln!("{}", style("Passwords do not match").red());
                continue;
            }

            break pwd;
        }
    };

    // Create the user
    create_user(client, CONTAINER_NAME, &username).await?;

    // Set password
    set_user_password(client, CONTAINER_NAME, &username, &password).await?;

    // Persist user credentials for rebuild/update restores
    persist_user(client, CONTAINER_NAME, &username).await?;

    // Update config - add username to users array
    let mut config = load_config_or_default()?;
    if !config.users.contains(&username) {
        config.users.push(username.clone());
        save_config(&config)?;
    }

    if args.print_password_only {
        println!("{password}");
        return Ok(());
    }

    // Display success
    if !quiet {
        println!(
            "{} User '{}' created successfully",
            style("Success:").green().bold(),
            username
        );

        if generated {
            print_generated_password(
                &password,
                "Save this password securely - it won't be shown again.",
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("ABC").is_ok());
        assert!(validate_username("opencode").is_ok());
    }

    #[test]
    fn test_validate_username_empty() {
        assert!(validate_username("").is_err());
    }

    #[test]
    fn test_validate_username_too_short() {
        assert!(validate_username("ab").is_err());
    }

    #[test]
    fn test_validate_username_too_long() {
        let long = "a".repeat(33);
        assert!(validate_username(&long).is_err());
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        assert!(validate_username("user@name").is_err());
        assert!(validate_username("user-name").is_err());
        assert!(validate_username("user name").is_err());
    }

    #[test]
    fn test_generate_random_password_length() {
        let password = generate_random_password();
        assert_eq!(password.len(), crate::passwords::password_length());
    }

    #[test]
    fn test_generate_random_password_alphanumeric() {
        let password = generate_random_password();
        assert!(password.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_random_password_uniqueness() {
        let p1 = generate_random_password();
        let p2 = generate_random_password();
        let p3 = generate_random_password();
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p1, p3);
    }
}
