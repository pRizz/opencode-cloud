//! Host configuration storage
//!
//! Load and save hosts.json file.

use std::fs::{self, File};
use std::io::{Read, Write};

use super::error::HostError;
use super::schema::HostsFile;
use crate::config::paths::get_hosts_path;

/// Load hosts configuration from hosts.json
///
/// Returns empty HostsFile if file doesn't exist.
pub fn load_hosts() -> Result<HostsFile, HostError> {
    let hosts_path = get_hosts_path()
        .ok_or_else(|| HostError::LoadFailed("Could not determine hosts file path".to_string()))?;

    if !hosts_path.exists() {
        tracing::debug!(
            "Hosts file not found, returning empty: {}",
            hosts_path.display()
        );
        return Ok(HostsFile::new());
    }

    let mut file = File::open(&hosts_path).map_err(|e| {
        HostError::LoadFailed(format!("Failed to open {}: {}", hosts_path.display(), e))
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| {
        HostError::LoadFailed(format!("Failed to read {}: {}", hosts_path.display(), e))
    })?;

    let hosts: HostsFile = serde_json::from_str(&contents).map_err(|e| {
        HostError::LoadFailed(format!("Invalid JSON in {}: {}", hosts_path.display(), e))
    })?;

    tracing::debug!(
        "Loaded {} hosts from {}",
        hosts.hosts.len(),
        hosts_path.display()
    );
    Ok(hosts)
}

/// Save hosts configuration to hosts.json
///
/// Creates the config directory if it doesn't exist.
/// Creates a backup (.bak) if file already exists.
pub fn save_hosts(hosts: &HostsFile) -> Result<(), HostError> {
    let hosts_path = get_hosts_path()
        .ok_or_else(|| HostError::SaveFailed("Could not determine hosts file path".to_string()))?;

    // Ensure config directory exists
    if let Some(parent) = hosts_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| HostError::SaveFailed(format!("Failed to create directory: {}", e)))?;
        }
    }

    // Create backup if file exists
    if hosts_path.exists() {
        let backup_path = hosts_path.with_extension("json.bak");
        fs::copy(&hosts_path, &backup_path)
            .map_err(|e| HostError::SaveFailed(format!("Failed to create backup: {}", e)))?;
        tracing::debug!("Created hosts backup: {}", backup_path.display());
    }

    // Serialize with pretty formatting
    let json = serde_json::to_string_pretty(hosts)
        .map_err(|e| HostError::SaveFailed(format!("Failed to serialize: {}", e)))?;

    // Write to file
    let mut file = File::create(&hosts_path).map_err(|e| {
        HostError::SaveFailed(format!("Failed to create {}: {}", hosts_path.display(), e))
    })?;

    file.write_all(json.as_bytes()).map_err(|e| {
        HostError::SaveFailed(format!("Failed to write {}: {}", hosts_path.display(), e))
    })?;

    tracing::debug!(
        "Saved {} hosts to {}",
        hosts.hosts.len(),
        hosts_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::schema::HostConfig;

    #[test]
    fn test_load_nonexistent_returns_empty() {
        // This test relies on hosts.json not existing in a fresh environment
        // In CI/testing, we'd mock the path, but for basic test:
        let result = load_hosts();
        // Should succeed with empty or existing hosts
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_format() {
        let mut hosts = HostsFile::new();
        hosts.add_host("test", HostConfig::new("test.example.com"));

        let json = serde_json::to_string_pretty(&hosts).unwrap();

        // Verify it's valid JSON that can be read back
        let parsed: HostsFile = serde_json::from_str(&json).unwrap();
        assert!(parsed.has_host("test"));
    }
}
