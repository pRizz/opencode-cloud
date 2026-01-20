//! Configuration schema for opencode-cloud
//!
//! Defines the structure and defaults for the config.json file.

use serde::{Deserialize, Serialize};

/// Main configuration structure for opencode-cloud
///
/// Serialized to/from `~/.config/opencode-cloud/config.json`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config file version for migrations
    pub version: u32,

    /// Port for the opencode web UI (default: 3000)
    #[serde(default = "default_opencode_web_port")]
    pub opencode_web_port: u16,

    /// Bind address (default: "localhost")
    /// Use "localhost" for local-only access (secure default)
    /// Use "0.0.0.0" for network access (requires explicit opt-in)
    #[serde(default = "default_bind")]
    pub bind: String,

    /// Auto-restart service on crash (default: true)
    #[serde(default = "default_auto_restart")]
    pub auto_restart: bool,

    /// Boot mode for service registration (default: "user")
    /// "user" - Service starts on user login (no root required)
    /// "system" - Service starts on boot (requires root)
    #[serde(default = "default_boot_mode")]
    pub boot_mode: String,

    /// Number of restart attempts on crash (default: 3)
    #[serde(default = "default_restart_retries")]
    pub restart_retries: u32,

    /// Seconds between restart attempts (default: 5)
    #[serde(default = "default_restart_delay")]
    pub restart_delay: u32,

    /// Username for opencode basic auth (default: None, triggers wizard)
    #[serde(default)]
    pub auth_username: Option<String>,

    /// Password for opencode basic auth (default: None, triggers wizard)
    #[serde(default)]
    pub auth_password: Option<String>,

    /// Environment variables passed to container (default: empty)
    /// Format: ["KEY=value", "KEY2=value2"]
    #[serde(default)]
    pub container_env: Vec<String>,
}

fn default_opencode_web_port() -> u16 {
    3000
}

fn default_bind() -> String {
    "localhost".to_string()
}

fn default_auto_restart() -> bool {
    true
}

fn default_boot_mode() -> String {
    "user".to_string()
}

fn default_restart_retries() -> u32 {
    3
}

fn default_restart_delay() -> u32 {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            opencode_web_port: default_opencode_web_port(),
            bind: default_bind(),
            auto_restart: default_auto_restart(),
            boot_mode: default_boot_mode(),
            restart_retries: default_restart_retries(),
            restart_delay: default_restart_delay(),
            auth_username: None,
            auth_password: None,
            container_env: Vec::new(),
        }
    }
}

impl Config {
    /// Create a new Config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if required auth credentials are configured
    ///
    /// Returns true if both auth_username and auth_password are Some and non-empty.
    /// This is used to determine if the setup wizard needs to run.
    pub fn has_required_auth(&self) -> bool {
        match (&self.auth_username, &self.auth_password) {
            (Some(username), Some(password)) => !username.is_empty() && !password.is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, 1);
        assert_eq!(config.opencode_web_port, 3000);
        assert_eq!(config.bind, "localhost");
        assert!(config.auto_restart);
        assert_eq!(config.boot_mode, "user");
        assert_eq!(config.restart_retries, 3);
        assert_eq!(config.restart_delay, 5);
        assert!(config.auth_username.is_none());
        assert!(config.auth_password.is_none());
        assert!(config.container_env.is_empty());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_deserialize_with_missing_optional_fields() {
        let json = r#"{"version": 1}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.opencode_web_port, 3000);
        assert_eq!(config.bind, "localhost");
        assert!(config.auto_restart);
        assert_eq!(config.boot_mode, "user");
        assert_eq!(config.restart_retries, 3);
        assert_eq!(config.restart_delay, 5);
        assert!(config.auth_username.is_none());
        assert!(config.auth_password.is_none());
        assert!(config.container_env.is_empty());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_with_service_fields() {
        let config = Config {
            version: 1,
            opencode_web_port: 9000,
            bind: "0.0.0.0".to_string(),
            auto_restart: false,
            boot_mode: "system".to_string(),
            restart_retries: 5,
            restart_delay: 10,
            auth_username: None,
            auth_password: None,
            container_env: Vec::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
        assert_eq!(parsed.boot_mode, "system");
        assert_eq!(parsed.restart_retries, 5);
        assert_eq!(parsed.restart_delay, 10);
    }

    #[test]
    fn test_reject_unknown_fields() {
        let json = r#"{"version": 1, "unknown_field": "value"}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_with_auth_fields() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            auth_password: Some("secret123".to_string()),
            container_env: vec!["FOO=bar".to_string(), "BAZ=qux".to_string()],
            ..Config::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
        assert_eq!(parsed.auth_username, Some("admin".to_string()));
        assert_eq!(parsed.auth_password, Some("secret123".to_string()));
        assert_eq!(parsed.container_env, vec!["FOO=bar", "BAZ=qux"]);
    }

    #[test]
    fn test_has_required_auth_returns_false_when_both_none() {
        let config = Config::default();
        assert!(!config.has_required_auth());
    }

    #[test]
    fn test_has_required_auth_returns_false_when_username_none() {
        let config = Config {
            auth_username: None,
            auth_password: Some("secret".to_string()),
            ..Config::default()
        };
        assert!(!config.has_required_auth());
    }

    #[test]
    fn test_has_required_auth_returns_false_when_password_none() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            auth_password: None,
            ..Config::default()
        };
        assert!(!config.has_required_auth());
    }

    #[test]
    fn test_has_required_auth_returns_false_when_username_empty() {
        let config = Config {
            auth_username: Some(String::new()),
            auth_password: Some("secret".to_string()),
            ..Config::default()
        };
        assert!(!config.has_required_auth());
    }

    #[test]
    fn test_has_required_auth_returns_false_when_password_empty() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            auth_password: Some(String::new()),
            ..Config::default()
        };
        assert!(!config.has_required_auth());
    }

    #[test]
    fn test_has_required_auth_returns_true_when_both_set() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            auth_password: Some("secret123".to_string()),
            ..Config::default()
        };
        assert!(config.has_required_auth());
    }
}
