//! SSH config file parsing and writing
//!
//! Parses ~/.ssh/config to auto-fill host settings and can write new entries.

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;

use ssh2_config_rs::{ParseRule, SshConfig};

use super::error::HostError;

/// Settings found in user's SSH config for a host
#[derive(Debug, Clone, Default)]
pub struct SshConfigMatch {
    /// User from SSH config
    pub user: Option<String>,
    /// Port from SSH config
    pub port: Option<u16>,
    /// Identity file path from SSH config
    pub identity_file: Option<String>,
    /// ProxyJump (jump host) from SSH config
    pub proxy_jump: Option<String>,
    /// Whether any match was found
    pub matched: bool,
}

impl SshConfigMatch {
    /// Check if any useful settings were found
    pub fn has_settings(&self) -> bool {
        self.user.is_some()
            || self.port.is_some()
            || self.identity_file.is_some()
            || self.proxy_jump.is_some()
    }

    /// Format found settings for display
    pub fn display_settings(&self) -> String {
        let mut parts = Vec::new();

        if let Some(user) = &self.user {
            parts.push(format!("User={user}"));
        }
        if let Some(port) = self.port {
            parts.push(format!("Port={port}"));
        }
        if let Some(key) = &self.identity_file {
            parts.push(format!("IdentityFile={key}"));
        }
        if let Some(jump) = &self.proxy_jump {
            parts.push(format!("ProxyJump={jump}"));
        }

        parts.join(", ")
    }
}

/// Get the path to the user's SSH config file
pub fn get_ssh_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh").join("config"))
}

/// Parse user's SSH config and query for a hostname
///
/// Returns settings found for the given hostname, applying SSH config
/// precedence rules (first match wins).
pub fn query_ssh_config(hostname: &str) -> Result<SshConfigMatch, HostError> {
    let config_path = match get_ssh_config_path() {
        Some(path) if path.exists() => path,
        _ => {
            tracing::debug!("No SSH config file found");
            return Ok(SshConfigMatch::default());
        }
    };

    let file = File::open(&config_path).map_err(|e| {
        HostError::SshConfigRead(format!("Failed to open {}: {}", config_path.display(), e))
    })?;

    let mut reader = BufReader::new(file);

    // Use ALLOW_UNKNOWN_FIELDS to be lenient with SSH config options we don't support
    let config = SshConfig::default()
        .parse(&mut reader, ParseRule::ALLOW_UNKNOWN_FIELDS)
        .map_err(|e| HostError::SshConfigRead(format!("Failed to parse SSH config: {e}")))?;

    // Query for the hostname
    let params = config.query(hostname);

    let mut result = SshConfigMatch {
        matched: true,
        ..Default::default()
    };

    // Extract relevant fields
    if let Some(user) = params.user {
        result.user = Some(user);
    }
    if let Some(port) = params.port {
        result.port = Some(port);
    }
    if let Some(files) = params.identity_file {
        // SSH config can have multiple identity files; take the first
        if let Some(first) = files.first() {
            result.identity_file = Some(first.to_string_lossy().to_string());
        }
    }
    if let Some(jump) = params.proxy_jump {
        // SSH config can have multiple jump hosts chained; join them
        if !jump.is_empty() {
            result.proxy_jump = Some(jump.join(","));
        }
    }

    // Check if we actually found anything useful
    if !result.has_settings() {
        result.matched = false;
    }

    Ok(result)
}

/// Write a new host entry to the user's SSH config file
///
/// Appends a Host block to ~/.ssh/config with the provided settings.
/// Creates the file and directory if they don't exist.
pub fn write_ssh_config_entry(
    alias: &str,
    hostname: &str,
    user: Option<&str>,
    port: Option<u16>,
    identity_file: Option<&str>,
    jump_host: Option<&str>,
) -> Result<PathBuf, HostError> {
    let config_path = get_ssh_config_path().ok_or_else(|| {
        HostError::SshConfigWrite("Could not determine home directory".to_string())
    })?;

    // Ensure .ssh directory exists with proper permissions
    if let Some(ssh_dir) = config_path.parent()
        && !ssh_dir.exists()
    {
        fs::create_dir_all(ssh_dir).map_err(|e| {
            HostError::SshConfigWrite(format!("Failed to create .ssh directory: {e}"))
        })?;

        // Set directory permissions to 700 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(ssh_dir, perms).map_err(|e| {
                HostError::SshConfigWrite(format!("Failed to set .ssh permissions: {e}"))
            })?;
        }
    }

    // Build the config entry
    let mut entry = String::new();
    entry.push_str(&format!("\n# Added by opencode-cloud for host '{alias}'\n"));
    entry.push_str(&format!("Host {alias}\n"));
    entry.push_str(&format!("    HostName {hostname}\n"));

    if let Some(u) = user {
        entry.push_str(&format!("    User {u}\n"));
    }
    if let Some(p) = port
        && p != 22
    {
        entry.push_str(&format!("    Port {p}\n"));
    }
    if let Some(key) = identity_file {
        entry.push_str(&format!("    IdentityFile {key}\n"));
    }
    if let Some(jump) = jump_host {
        entry.push_str(&format!("    ProxyJump {jump}\n"));
    }

    // Append to config file (create if doesn't exist)
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)
        .map_err(|e| {
            HostError::SshConfigWrite(format!("Failed to open {}: {}", config_path.display(), e))
        })?;

    // Set file permissions to 600 on Unix if we just created it
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = file
            .metadata()
            .map_err(|e| HostError::SshConfigWrite(format!("Failed to get file metadata: {e}")))?;
        if metadata.len() == 0 {
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&config_path, perms).map_err(|e| {
                HostError::SshConfigWrite(format!("Failed to set config permissions: {e}"))
            })?;
        }
    }

    file.write_all(entry.as_bytes()).map_err(|e| {
        HostError::SshConfigWrite(format!(
            "Failed to write to {}: {}",
            config_path.display(),
            e
        ))
    })?;

    tracing::info!(
        "Added host '{}' to SSH config at {}",
        alias,
        config_path.display()
    );

    Ok(config_path)
}

/// Check if a host alias already exists in SSH config
pub fn host_exists_in_ssh_config(alias: &str) -> bool {
    let config_path = match get_ssh_config_path() {
        Some(path) if path.exists() => path,
        _ => return false,
    };

    let Ok(file) = File::open(&config_path) else {
        return false;
    };

    let mut reader = BufReader::new(file);

    let Ok(config) = SshConfig::default().parse(&mut reader, ParseRule::ALLOW_UNKNOWN_FIELDS)
    else {
        return false;
    };

    // Query returns default params if not found, so we check if hostname is set
    let params = config.query(alias);
    params.host_name.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_match_display() {
        let m = SshConfigMatch {
            user: Some("ubuntu".to_string()),
            port: Some(2222),
            identity_file: Some("~/.ssh/mykey.pem".to_string()),
            proxy_jump: None,
            matched: true,
        };

        let display = m.display_settings();
        assert!(display.contains("User=ubuntu"));
        assert!(display.contains("Port=2222"));
        assert!(display.contains("IdentityFile=~/.ssh/mykey.pem"));
    }

    #[test]
    fn test_ssh_config_match_has_settings() {
        let empty = SshConfigMatch::default();
        assert!(!empty.has_settings());

        let with_user = SshConfigMatch {
            user: Some("test".to_string()),
            ..Default::default()
        };
        assert!(with_user.has_settings());
    }

    #[test]
    fn test_get_ssh_config_path() {
        let path = get_ssh_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with(".ssh/config"));
    }
}
