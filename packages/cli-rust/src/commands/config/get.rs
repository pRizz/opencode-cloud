//! Config get subcommand
//!
//! Retrieves a single configuration value by key.

use anyhow::{Result, bail};
use opencode_cloud_core::Config;

/// Get a single configuration value
///
/// Outputs just the value (no formatting) for scripting.
/// Passwords are always masked for security.
pub fn cmd_config_get(config: &Config, key: &str, _quiet: bool) -> Result<()> {
    // Normalize key (support both short and full forms)
    let value = match key.to_lowercase().as_str() {
        "version" => config.version.to_string(),
        "port" | "opencode_web_port" => config.opencode_web_port.to_string(),
        "bind" | "hostname" => config.bind.clone(),
        "bind_address" | "host" => config.bind_address.clone(),
        "auto_restart" => config.auto_restart.to_string(),
        "boot_mode" => config.boot_mode.clone(),
        "restart_retries" => config.restart_retries.to_string(),
        "restart_delay" => config.restart_delay.to_string(),
        "username" | "auth_username" => format_optional(&config.auth_username),
        "password" | "auth_password" => {
            // Never reveal actual password
            match &config.auth_password {
                Some(s) if !s.is_empty() => "********".to_string(),
                _ => String::new(),
            }
        }
        "env" | "container_env" => {
            // Output as JSON array for scripting
            serde_json::to_string(&config.container_env)?
        }
        _ => {
            bail!(
                "Unknown configuration key: {key}\n\n\
                Valid keys:\n  \
                  version\n  \
                  port / opencode_web_port\n  \
                  bind / hostname\n  \
                  bind_address / host\n  \
                  auto_restart\n  \
                  boot_mode\n  \
                  restart_retries\n  \
                  restart_delay\n  \
                  username / auth_username\n  \
                  password / auth_password\n  \
                  env / container_env"
            );
        }
    };

    println!("{value}");
    Ok(())
}

/// Format an optional string, returning empty string if None
fn format_optional(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_optional_with_value() {
        assert_eq!(format_optional(&Some("test".to_string())), "test");
    }

    #[test]
    fn test_format_optional_with_none() {
        assert_eq!(format_optional(&None), "");
    }

    #[test]
    fn test_format_optional_with_empty() {
        assert_eq!(format_optional(&Some(String::new())), "");
    }
}
