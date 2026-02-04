//! Host configuration schema
//!
//! Data structures for storing remote host configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a remote host
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
    /// SSH hostname or IP address
    pub hostname: String,

    /// SSH username (default: current user from whoami)
    #[serde(default = "default_user")]
    pub user: String,

    /// SSH port (default: 22)
    #[serde(default)]
    pub port: Option<u16>,

    /// Path to SSH identity file (private key)
    #[serde(default)]
    pub identity_file: Option<String>,

    /// Jump host for ProxyJump (user@host:port format)
    #[serde(default)]
    pub jump_host: Option<String>,

    /// Organization groups/tags for this host
    #[serde(default)]
    pub groups: Vec<String>,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

fn default_user() -> String {
    whoami::username().unwrap_or_else(|_| "user".to_string())
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            user: default_user(),
            port: None,
            identity_file: None,
            jump_host: None,
            groups: Vec::new(),
            description: None,
        }
    }
}

impl HostConfig {
    /// Create a new host config with just hostname
    pub fn new(hostname: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            ..Default::default()
        }
    }

    /// Builder pattern: set user
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = user.into();
        self
    }

    /// Builder pattern: set port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Builder pattern: set identity file
    pub fn with_identity_file(mut self, path: impl Into<String>) -> Self {
        self.identity_file = Some(path.into());
        self
    }

    /// Builder pattern: set jump host
    pub fn with_jump_host(mut self, jump: impl Into<String>) -> Self {
        self.jump_host = Some(jump.into());
        self
    }

    /// Builder pattern: add group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Builder pattern: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Get SSH command arguments for this host
    ///
    /// Returns arguments for port, identity file, jump host, and target (user@hostname).
    /// Does NOT include standard options like BatchMode or ConnectTimeout.
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Port (if specified)
        if let Some(port) = self.port {
            args.push("-p".to_string());
            args.push(port.to_string());
        }

        // Identity file
        if let Some(key) = &self.identity_file {
            args.push("-i".to_string());
            args.push(key.clone());
        }

        // Jump host
        if let Some(jump) = &self.jump_host {
            args.push("-J".to_string());
            args.push(jump.clone());
        }

        // Target: user@hostname
        args.push(format!("{}@{}", self.user, self.hostname));

        args
    }

    /// Format the effective SSH command for display
    ///
    /// Returns a human-readable SSH command string showing how the connection
    /// will be made, useful for debugging and user feedback.
    pub fn format_ssh_command(&self) -> String {
        let mut parts = vec!["ssh".to_string()];

        // Port (if non-default)
        if let Some(port) = self.port
            && port != 22
        {
            parts.push(format!("-p {port}"));
        }

        // Identity file
        if let Some(key) = &self.identity_file {
            parts.push(format!("-i {key}"));
        }

        // Jump host
        if let Some(jump) = &self.jump_host {
            parts.push(format!("-J {jump}"));
        }

        // Target: user@hostname
        parts.push(format!("{}@{}", self.user, self.hostname));

        parts.join(" ")
    }
}

/// Root structure for hosts.json file
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostsFile {
    /// Schema version for future migrations
    #[serde(default = "default_version")]
    pub version: u32,

    /// Default host name (None = local Docker)
    #[serde(default)]
    pub default_host: Option<String>,

    /// Map of host name to configuration
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
}

fn default_version() -> u32 {
    1
}

impl HostsFile {
    /// Create empty hosts file
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a host
    pub fn add_host(&mut self, name: impl Into<String>, config: HostConfig) {
        self.hosts.insert(name.into(), config);
    }

    /// Remove a host
    pub fn remove_host(&mut self, name: &str) -> Option<HostConfig> {
        // Clear default if removing the default host
        if self.default_host.as_deref() == Some(name) {
            self.default_host = None;
        }
        self.hosts.remove(name)
    }

    /// Get a host by name
    pub fn get_host(&self, name: &str) -> Option<&HostConfig> {
        self.hosts.get(name)
    }

    /// Get mutable reference to a host
    pub fn get_host_mut(&mut self, name: &str) -> Option<&mut HostConfig> {
        self.hosts.get_mut(name)
    }

    /// Check if host exists
    pub fn has_host(&self, name: &str) -> bool {
        self.hosts.contains_key(name)
    }

    /// Set the default host
    pub fn set_default(&mut self, name: Option<String>) {
        self.default_host = name;
    }

    /// Get list of host names
    pub fn host_names(&self) -> Vec<&str> {
        self.hosts.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_config_defaults() {
        let config = HostConfig::default();
        assert!(config.hostname.is_empty());
        assert!(!config.user.is_empty()); // Should be current user
        assert!(config.port.is_none());
        assert!(config.identity_file.is_none());
        assert!(config.jump_host.is_none());
        assert!(config.groups.is_empty());
        assert!(config.description.is_none());
    }

    #[test]
    fn test_host_config_builder() {
        let config = HostConfig::new("example.com")
            .with_user("admin")
            .with_port(2222)
            .with_identity_file("~/.ssh/prod_key")
            .with_group("production");

        assert_eq!(config.hostname, "example.com");
        assert_eq!(config.user, "admin");
        assert_eq!(config.port, Some(2222));
        assert_eq!(config.identity_file, Some("~/.ssh/prod_key".to_string()));
        assert_eq!(config.groups, vec!["production"]);
    }

    #[test]
    fn test_hosts_file_operations() {
        let mut hosts = HostsFile::new();
        assert!(hosts.hosts.is_empty());

        // Add host
        hosts.add_host("prod-1", HostConfig::new("prod1.example.com"));
        assert!(hosts.has_host("prod-1"));
        assert!(!hosts.has_host("prod-2"));

        // Set default
        hosts.set_default(Some("prod-1".to_string()));
        assert_eq!(hosts.default_host, Some("prod-1".to_string()));

        // Remove host clears default
        hosts.remove_host("prod-1");
        assert!(!hosts.has_host("prod-1"));
        assert!(hosts.default_host.is_none());
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut hosts = HostsFile::new();
        hosts.add_host(
            "test",
            HostConfig::new("test.example.com")
                .with_user("testuser")
                .with_port(22),
        );

        let json = serde_json::to_string_pretty(&hosts).unwrap();
        let parsed: HostsFile = serde_json::from_str(&json).unwrap();

        assert_eq!(hosts, parsed);
    }

    #[test]
    fn test_deserialize_minimal() {
        // Minimal JSON should work with defaults
        let json = r#"{"version": 1}"#;
        let hosts: HostsFile = serde_json::from_str(json).unwrap();
        assert_eq!(hosts.version, 1);
        assert!(hosts.hosts.is_empty());
        assert!(hosts.default_host.is_none());
    }
}
