//! Config show subcommand
//!
//! Displays current configuration in table or JSON format.

use anyhow::Result;
use comfy_table::{Cell, Table};
use opencode_cloud_core::{Config, config};

/// Show current configuration
///
/// Displays all configuration values in a formatted table.
/// Passwords are masked for security.
pub fn cmd_config_show(config: &Config, json: bool, _quiet: bool) -> Result<()> {
    if json {
        // Output as JSON with password masked
        let masked_config = MaskedConfig::from(config);
        let output = serde_json::to_string_pretty(&masked_config)?;
        println!("{output}");
    } else {
        // Output as table
        let mut table = Table::new();
        table.set_header(vec!["Key", "Value"]);

        // Add each configuration value
        table.add_row(vec![
            Cell::new("version"),
            Cell::new(config.version.to_string()),
        ]);
        table.add_row(vec![
            Cell::new("opencode_web_port"),
            Cell::new(config.opencode_web_port.to_string()),
        ]);
        table.add_row(vec![Cell::new("bind"), Cell::new(&config.bind)]);
        table.add_row(vec![
            Cell::new("auto_restart"),
            Cell::new(config.auto_restart.to_string()),
        ]);
        table.add_row(vec![Cell::new("boot_mode"), Cell::new(&config.boot_mode)]);
        table.add_row(vec![
            Cell::new("restart_retries"),
            Cell::new(config.restart_retries.to_string()),
        ]);
        table.add_row(vec![
            Cell::new("restart_delay"),
            Cell::new(config.restart_delay.to_string()),
        ]);
        table.add_row(vec![
            Cell::new("auth_username"),
            Cell::new(format_optional(&config.auth_username)),
        ]);
        table.add_row(vec![
            Cell::new("auth_password"),
            Cell::new(format_password(&config.auth_password)),
        ]);
        table.add_row(vec![
            Cell::new("container_env"),
            Cell::new(format_env_vars(&config.container_env)),
        ]);

        println!("{table}");

        // Show config file location
        if let Some(path) = config::paths::get_config_path() {
            println!();
            println!("Config file: {}", path.display());
        }
    }

    Ok(())
}

/// Format an optional string for display
fn format_optional(value: &Option<String>) -> String {
    match value {
        Some(s) if !s.is_empty() => s.clone(),
        _ => "(not set)".to_string(),
    }
}

/// Format a password for display (masked)
fn format_password(value: &Option<String>) -> String {
    match value {
        Some(s) if !s.is_empty() => "********".to_string(),
        _ => "(not set)".to_string(),
    }
}

/// Format environment variables for display
fn format_env_vars(vars: &[String]) -> String {
    if vars.is_empty() {
        "(none)".to_string()
    } else {
        vars.join(", ")
    }
}

/// Config struct with password masked for JSON output
#[derive(serde::Serialize)]
struct MaskedConfig {
    version: u32,
    opencode_web_port: u16,
    bind: String,
    auto_restart: bool,
    boot_mode: String,
    restart_retries: u32,
    restart_delay: u32,
    auth_username: Option<String>,
    auth_password: Option<String>,
    container_env: Vec<String>,
}

impl From<&Config> for MaskedConfig {
    fn from(config: &Config) -> Self {
        Self {
            version: config.version,
            opencode_web_port: config.opencode_web_port,
            bind: config.bind.clone(),
            auto_restart: config.auto_restart,
            boot_mode: config.boot_mode.clone(),
            restart_retries: config.restart_retries,
            restart_delay: config.restart_delay,
            auth_username: config.auth_username.clone(),
            // Mask password in JSON output too
            auth_password: config.auth_password.as_ref().map(|s| {
                if s.is_empty() {
                    String::new()
                } else {
                    "********".to_string()
                }
            }),
            container_env: config.container_env.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_optional_with_value() {
        assert_eq!(format_optional(&Some("admin".to_string())), "admin");
    }

    #[test]
    fn test_format_optional_with_empty() {
        assert_eq!(format_optional(&Some(String::new())), "(not set)");
    }

    #[test]
    fn test_format_optional_with_none() {
        assert_eq!(format_optional(&None), "(not set)");
    }

    #[test]
    fn test_format_password_masks_value() {
        assert_eq!(format_password(&Some("secret123".to_string())), "********");
    }

    #[test]
    fn test_format_password_shows_not_set_when_empty() {
        assert_eq!(format_password(&Some(String::new())), "(not set)");
    }

    #[test]
    fn test_format_password_shows_not_set_when_none() {
        assert_eq!(format_password(&None), "(not set)");
    }

    #[test]
    fn test_format_env_vars_with_values() {
        let vars = vec!["FOO=bar".to_string(), "BAZ=qux".to_string()];
        assert_eq!(format_env_vars(&vars), "FOO=bar, BAZ=qux");
    }

    #[test]
    fn test_format_env_vars_empty() {
        let vars: Vec<String> = vec![];
        assert_eq!(format_env_vars(&vars), "(none)");
    }

    #[test]
    fn test_masked_config_hides_password() {
        let config = Config {
            auth_password: Some("secret".to_string()),
            ..Config::default()
        };
        let masked = MaskedConfig::from(&config);
        assert_eq!(masked.auth_password, Some("********".to_string()));
    }

    #[test]
    fn test_masked_config_preserves_username() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            ..Config::default()
        };
        let masked = MaskedConfig::from(&config);
        assert_eq!(masked.auth_username, Some("admin".to_string()));
    }
}
