//! Config set subcommand
//!
//! Sets a single configuration value.

use anyhow::{Result, bail};
use console::style;
use dialoguer::Password;
use opencode_cloud_core::config::validate_bind_address;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, container_is_running};
use opencode_cloud_core::{load_config, save_config};

/// Set a configuration value
///
/// Special handling for password: prompts interactively if value is None.
/// Returns error if password value is provided on command line (security risk).
pub fn cmd_config_set(key: &str, value: Option<&str>, quiet: bool) -> Result<()> {
    let mut config = load_config()?;
    let normalized_key = key.to_lowercase();

    // Display value for output (password is masked)
    let display_value: String;

    match normalized_key.as_str() {
        "password" | "auth_password" => {
            // Security: never accept password via command line argument
            if value.is_some() {
                bail!(
                    "Password cannot be set via command line for security.\n\
                     Use: occ config set password  (will prompt securely)"
                );
            }

            // Prompt for password interactively
            let password = Password::new()
                .with_prompt("New password")
                .with_confirmation("Confirm password", "Passwords do not match")
                .interact()?;

            config.auth_password = Some(password);
            display_value = "********".to_string();
        }

        "port" | "opencode_web_port" => {
            let val = require_value(value, key)?;
            let port: u16 = val.parse().map_err(|_| {
                anyhow::anyhow!("Invalid port number: {val}. Must be a number between 1-65535.")
            })?;
            config.opencode_web_port = port;
            display_value = port.to_string();
        }

        "bind" | "hostname" => {
            let val = require_value(value, key)?;
            config.bind = val.to_string();
            display_value = val.to_string();
        }

        "bind_address" | "host" => {
            let val = require_value(value, key)?;

            // Validate the address
            validate_bind_address(val).map_err(|_| {
                anyhow::anyhow!(
                    "Invalid address: {val}\n\
                     Valid examples: 127.0.0.1, ::1, 0.0.0.0, ::, or localhost"
                )
            })?;

            // Check for network exposure and show warning
            if val == "0.0.0.0" || val == "::" {
                eprintln!();
                eprintln!(
                    "{} {}",
                    style("WARNING:").yellow().bold(),
                    style("Network exposure enabled!").yellow()
                );
                eprintln!();
                eprintln!(
                    "Binding to {} exposes the service to all network interfaces.",
                    style(val).cyan()
                );
                eprintln!("Anyone on your network can access the opencode web UI.");
                eprintln!();
                eprintln!("{}", style("Recommendations:").bold());
                eprintln!("  - Ensure strong authentication is configured (occ user add)");
                eprintln!("  - Consider using a firewall to restrict access");
                eprintln!("  - For internet exposure, use a reverse proxy with TLS");
                eprintln!();
                eprintln!(
                    "To bind to localhost only: {}",
                    style("occ config set bind_address 127.0.0.1").cyan()
                );
                eprintln!();
            }

            config.bind_address = val.to_string();
            display_value = val.to_string();
        }

        "username" | "auth_username" => {
            let val = require_value(value, key)?;
            validate_username(val)?;
            config.auth_username = Some(val.to_string());
            display_value = val.to_string();
        }

        "auto_restart" => {
            let val = require_value(value, key)?;
            let parsed = parse_bool(val).ok_or_else(|| {
                anyhow::anyhow!("Invalid boolean value: {val}. Use: true/false, yes/no, or 1/0")
            })?;
            config.auto_restart = parsed;
            display_value = parsed.to_string();
        }

        "boot_mode" => {
            let val = require_value(value, key)?;
            if val != "user" && val != "system" {
                bail!("Invalid boot_mode: {val}. Must be 'user' or 'system'.");
            }
            config.boot_mode = val.to_string();
            display_value = val.to_string();
        }

        "restart_retries" => {
            let val = require_value(value, key)?;
            let retries: u32 = val.parse().map_err(|_| {
                anyhow::anyhow!("Invalid restart_retries: {val}. Must be a positive integer.")
            })?;
            config.restart_retries = retries;
            display_value = retries.to_string();
        }

        "restart_delay" => {
            let val = require_value(value, key)?;
            let delay: u32 = val.parse().map_err(|_| {
                anyhow::anyhow!(
                    "Invalid restart_delay: {val}. Must be a positive integer (seconds)."
                )
            })?;
            config.restart_delay = delay;
            display_value = delay.to_string();
        }

        _ => {
            bail!(
                "Unknown configuration key: {key}\n\n\
                Valid keys:\n  \
                  port / opencode_web_port\n  \
                  bind / hostname\n  \
                  bind_address / host\n  \
                  username / auth_username\n  \
                  password / auth_password\n  \
                  auto_restart\n  \
                  boot_mode\n  \
                  restart_retries\n  \
                  restart_delay\n\n\
                For environment variables, use: occ config env set KEY=value"
            );
        }
    }

    // Save the config
    save_config(&config)?;

    // Check if service is running and warn
    if !quiet {
        if let Ok(true) = check_container_running() {
            eprintln!(
                "{} Restart required for changes to take effect",
                style("Warning:").yellow().bold()
            );
        }
    }

    if !quiet {
        println!(
            "{} Set {} = {}",
            style("Success:").green().bold(),
            key,
            display_value
        );
    }

    Ok(())
}

/// Require a value for non-password keys
fn require_value<'a>(value: Option<&'a str>, key: &str) -> Result<&'a str> {
    value.ok_or_else(|| {
        anyhow::anyhow!("Value required for key '{key}'.\nUsage: occ config set {key} <value>")
    })
}

/// Validate username according to rules
/// - Non-empty
/// - 3-32 characters
/// - Alphanumeric + underscore only
fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        bail!("Username cannot be empty");
    }
    if username.len() < 3 {
        bail!("Username must be at least 3 characters");
    }
    if username.len() > 32 {
        bail!("Username must be at most 32 characters");
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        bail!("Username must contain only alphanumeric characters and underscores");
    }
    Ok(())
}

/// Parse boolean from various string representations
fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" => Some(true),
        "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// Check if the container is running (synchronous wrapper)
fn check_container_running() -> Result<bool> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = DockerClient::new()?;
        container_is_running(&client, CONTAINER_NAME)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("ABC").is_ok());
        assert!(validate_username("a_b_c_d_e_f_g_h_i_j_k_l_m_n_").is_ok()); // 32 chars
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
    fn test_parse_bool_true_variants() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("True"), Some(true));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool("YES"), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
    }

    #[test]
    fn test_parse_bool_false_variants() {
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("False"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("NO"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
    }

    #[test]
    fn test_parse_bool_invalid() {
        assert_eq!(parse_bool("maybe"), None);
        assert_eq!(parse_bool("2"), None);
        assert_eq!(parse_bool(""), None);
    }
}
