//! Configuration management for opencode-cloud
//!
//! Handles loading, saving, and validating the JSONC configuration file.
//! Creates default config if missing, validates against schema.

pub mod paths;
pub mod schema;
pub mod validation;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use jsonc_parser::parse_to_serde_value;

pub use paths::{get_config_dir, get_config_path, get_data_dir, get_hosts_path, get_pid_path};
pub use schema::{Config, validate_bind_address};
pub use validation::{
    ValidationError, ValidationWarning, display_validation_error, display_validation_warning,
    validate_config,
};

/// Ensure the config directory exists
///
/// Creates `~/.config/opencode-cloud/` if it doesn't exist.
/// Returns the path to the config directory.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir =
        get_config_dir().ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
        tracing::info!("Created config directory: {}", config_dir.display());
    }

    Ok(config_dir)
}

/// Ensure the data directory exists
///
/// Creates `~/.local/share/opencode-cloud/` if it doesn't exist.
/// Returns the path to the data directory.
pub fn ensure_data_dir() -> Result<PathBuf> {
    let data_dir =
        get_data_dir().ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;
        tracing::info!("Created data directory: {}", data_dir.display());
    }

    Ok(data_dir)
}

/// Load configuration from the config file
///
/// If the config file doesn't exist, creates a new one with default values.
/// Supports JSONC (JSON with comments).
/// Rejects unknown fields for strict validation.
pub fn load_config() -> Result<Config> {
    let config_path =
        get_config_path().ok_or_else(|| anyhow::anyhow!("Could not determine config file path"))?;

    if !config_path.exists() {
        // Create default config
        tracing::info!(
            "Config file not found, creating default at: {}",
            config_path.display()
        );
        let config = Config::default();
        save_config(&config)?;
        return Ok(config);
    }

    // Read the file
    let mut file = File::open(&config_path)
        .with_context(|| format!("Failed to open config file: {}", config_path.display()))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    // Parse JSONC (JSON with comments)
    let mut parsed_value = parse_to_serde_value(&contents, &Default::default())
        .map_err(|e| anyhow::anyhow!("Invalid JSONC in config file: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("Config file is empty"))?;

    // Drop deprecated keys that were removed from the schema.
    if let Some(obj) = parsed_value.as_object_mut() {
        obj.remove("opencode_commit");
    }

    // Deserialize into Config struct (deny_unknown_fields will reject unknown keys)
    let config: Config = serde_json::from_value(parsed_value).with_context(|| {
        format!(
            "Invalid configuration in {}. Check for unknown fields or invalid values.",
            config_path.display()
        )
    })?;

    Ok(config)
}

/// Save configuration to the config file
///
/// Creates a backup of the existing config (config.json.bak) before overwriting.
/// Ensures the config directory exists.
pub fn save_config(config: &Config) -> Result<()> {
    ensure_config_dir()?;

    let config_path =
        get_config_path().ok_or_else(|| anyhow::anyhow!("Could not determine config file path"))?;

    // Create backup if file exists
    if config_path.exists() {
        let backup_path = config_path.with_extension("json.bak");
        fs::copy(&config_path, &backup_path)
            .with_context(|| format!("Failed to create backup at: {}", backup_path.display()))?;
        tracing::debug!("Created config backup: {}", backup_path.display());
    }

    // Serialize with pretty formatting
    let json = serde_json::to_string_pretty(config).context("Failed to serialize configuration")?;

    // Write to file
    let mut file = File::create(&config_path)
        .with_context(|| format!("Failed to create config file: {}", config_path.display()))?;

    file.write_all(json.as_bytes())
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    tracing::debug!("Saved config to: {}", config_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_resolution_returns_values() {
        // Verify path functions return Some on supported platforms
        assert!(get_config_dir().is_some());
        assert!(get_data_dir().is_some());
        assert!(get_config_path().is_some());
        assert!(get_pid_path().is_some());
    }

    #[test]
    fn test_paths_end_with_expected_names() {
        let config_dir = get_config_dir().unwrap();
        assert!(config_dir.ends_with("opencode-cloud"));

        let data_dir = get_data_dir().unwrap();
        assert!(data_dir.ends_with("opencode-cloud"));

        let config_path = get_config_path().unwrap();
        assert!(config_path.ends_with("config.json"));

        let pid_path = get_pid_path().unwrap();
        assert!(pid_path.ends_with("opencode-cloud.pid"));
    }

    // Note: Integration tests for load_config/save_config that modify the real
    // filesystem are run via CLI commands rather than unit tests to avoid
    // test isolation issues with environment variable manipulation in Rust 2024.
}
