//! Config env subcommand
//!
//! Manages container environment variables.

use anyhow::{Result, bail};
use clap::Subcommand;
use console::style;
use opencode_cloud_core::{load_config_or_default, save_config};

/// Environment variable management subcommands
#[derive(Subcommand)]
pub enum EnvCommands {
    /// Set or update an environment variable (format: KEY=value)
    Set {
        /// Environment variable in KEY=value format
        env_var: String,
    },
    /// List all configured environment variables
    List,
    /// Remove an environment variable
    Remove {
        /// Environment variable key to remove
        key: String,
    },
}

/// Handle config env subcommand
///
/// Routes to the appropriate handler based on the env subcommand.
pub fn cmd_config_env(cmd: EnvCommands, quiet: bool) -> Result<()> {
    match cmd {
        EnvCommands::Set { env_var } => cmd_env_set(&env_var, quiet),
        EnvCommands::List => cmd_env_list(quiet),
        EnvCommands::Remove { key } => cmd_env_remove(&key, quiet),
    }
}

/// Set or update an environment variable
fn cmd_env_set(env_var: &str, quiet: bool) -> Result<()> {
    // Validate format: must contain '='
    let Some(eq_pos) = env_var.find('=') else {
        bail!("Format must be KEY=value\n\nExample: occ config env set FOO=bar");
    };

    // Extract key (before first '=')
    let key = &env_var[..eq_pos];
    if key.is_empty() {
        bail!("Environment variable key cannot be empty");
    }

    // Load config
    let mut config = load_config_or_default()?;

    // Remove any existing entry with the same key
    let key_prefix = format!("{key}=");
    config.container_env.retain(|e| !e.starts_with(&key_prefix));

    // Add new entry
    config.container_env.push(env_var.to_string());

    // Save config
    save_config(&config)?;

    if !quiet {
        println!(
            "{} Set environment variable: {}",
            style("Success:").green().bold(),
            key
        );
    }

    Ok(())
}

/// List all configured environment variables
fn cmd_env_list(quiet: bool) -> Result<()> {
    let config = load_config_or_default()?;

    if config.container_env.is_empty() {
        if !quiet {
            println!("(no environment variables configured)");
        }
        return Ok(());
    }

    for env_var in &config.container_env {
        println!("  {env_var}");
    }

    if !quiet {
        println!();
        println!("{} environment variable(s)", config.container_env.len());
    }

    Ok(())
}

/// Remove an environment variable
fn cmd_env_remove(key: &str, quiet: bool) -> Result<()> {
    let mut config = load_config_or_default()?;

    // Check if any entry matches
    let key_prefix = format!("{key}=");
    let found = config
        .container_env
        .iter()
        .any(|e| e.starts_with(&key_prefix));

    if !found {
        bail!("Environment variable not found: {key}");
    }

    // Remove matching entry
    config.container_env.retain(|e| !e.starts_with(&key_prefix));

    // Save config
    save_config(&config)?;

    if !quiet {
        println!(
            "{} Removed environment variable: {}",
            style("Success:").green().bold(),
            key
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_key_extraction_from_env_var() {
        let env_var = "FOO=bar";
        let eq_pos = env_var.find('=').unwrap();
        let key = &env_var[..eq_pos];
        assert_eq!(key, "FOO");
    }

    #[test]
    fn test_key_extraction_with_equals_in_value() {
        let env_var = "FOO=bar=baz";
        let eq_pos = env_var.find('=').unwrap();
        let key = &env_var[..eq_pos];
        assert_eq!(key, "FOO");
    }

    #[test]
    fn test_key_prefix_matching() {
        let entries = ["FOO=bar".to_string(), "BAR=baz".to_string()];
        let key_prefix = "FOO=";
        let remaining: Vec<_> = entries
            .iter()
            .filter(|e| !e.starts_with(key_prefix))
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], "BAR=baz");
    }
}
