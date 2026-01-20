//! Configuration schema for opencode-cloud
//!
//! Defines the structure and defaults for the config.json file.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

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

    /// Bind address for opencode web UI (default: "127.0.0.1")
    /// Use "0.0.0.0" or "::" for network exposure (requires explicit opt-in)
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Trust proxy headers (X-Forwarded-For, etc.) for load balancer deployments
    #[serde(default)]
    pub trust_proxy: bool,

    /// Allow unauthenticated access when network exposed
    /// Requires double confirmation on first start
    #[serde(default)]
    pub allow_unauthenticated_network: bool,

    /// Maximum auth attempts before rate limiting
    #[serde(default = "default_rate_limit_attempts")]
    pub rate_limit_attempts: u32,

    /// Rate limit window in seconds
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window_seconds: u32,

    /// List of usernames configured in container (for persistence tracking)
    /// Passwords are NOT stored here - only in container's /etc/shadow
    #[serde(default)]
    pub users: Vec<String>,
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

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_rate_limit_attempts() -> u32 {
    5
}

fn default_rate_limit_window() -> u32 {
    60
}

/// Validate and parse a bind address string
///
/// Accepts:
/// - IPv4 addresses: "127.0.0.1", "0.0.0.0"
/// - IPv6 addresses: "::1", "::"
/// - Bracketed IPv6: "[::1]"
/// - "localhost" (resolves to 127.0.0.1)
///
/// Returns the parsed IpAddr or an error message.
pub fn validate_bind_address(addr: &str) -> Result<IpAddr, String> {
    let trimmed = addr.trim();

    // Handle "localhost" as special case
    if trimmed.eq_ignore_ascii_case("localhost") {
        return Ok(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    // Strip brackets from IPv6 addresses like "[::1]"
    let stripped = if trimmed.starts_with('[') && trimmed.ends_with(']') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    stripped.parse::<IpAddr>().map_err(|_| {
        format!(
            "Invalid IP address: '{}'. Use 127.0.0.1, ::1, 0.0.0.0, ::, or localhost",
            addr
        )
    })
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
            bind_address: default_bind_address(),
            trust_proxy: false,
            allow_unauthenticated_network: false,
            rate_limit_attempts: default_rate_limit_attempts(),
            rate_limit_window_seconds: default_rate_limit_window(),
            users: Vec::new(),
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
    /// Returns true if:
    /// - Both auth_username and auth_password are Some and non-empty (legacy), OR
    /// - The users array is non-empty (PAM-based auth)
    ///
    /// This is used to determine if the setup wizard needs to run.
    pub fn has_required_auth(&self) -> bool {
        // New PAM-based auth: users array
        if !self.users.is_empty() {
            return true;
        }

        // Legacy basic auth: username/password
        match (&self.auth_username, &self.auth_password) {
            (Some(username), Some(password)) => !username.is_empty() && !password.is_empty(),
            _ => false,
        }
    }

    /// Check if the bind address exposes the service to the network
    ///
    /// Returns true if bind_address is "0.0.0.0" (IPv4 all interfaces) or
    /// "::" (IPv6 all interfaces).
    pub fn is_network_exposed(&self) -> bool {
        match validate_bind_address(&self.bind_address) {
            Ok(IpAddr::V4(ip)) => ip.is_unspecified(),
            Ok(IpAddr::V6(ip)) => ip.is_unspecified(),
            Err(_) => false, // Invalid addresses are not considered exposed
        }
    }

    /// Check if the bind address is localhost-only
    ///
    /// Returns true if bind_address is "127.0.0.1", "::1", or "localhost".
    pub fn is_localhost(&self) -> bool {
        match validate_bind_address(&self.bind_address) {
            Ok(ip) => ip.is_loopback(),
            Err(_) => {
                // Also check for "localhost" string directly
                self.bind_address.eq_ignore_ascii_case("localhost")
            }
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
        // Security fields
        assert_eq!(config.bind_address, "127.0.0.1");
        assert!(!config.trust_proxy);
        assert!(!config.allow_unauthenticated_network);
        assert_eq!(config.rate_limit_attempts, 5);
        assert_eq!(config.rate_limit_window_seconds, 60);
        assert!(config.users.is_empty());
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
        // Security fields should have defaults
        assert_eq!(config.bind_address, "127.0.0.1");
        assert!(!config.trust_proxy);
        assert!(!config.allow_unauthenticated_network);
        assert_eq!(config.rate_limit_attempts, 5);
        assert_eq!(config.rate_limit_window_seconds, 60);
        assert!(config.users.is_empty());
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
            bind_address: "0.0.0.0".to_string(),
            trust_proxy: true,
            allow_unauthenticated_network: false,
            rate_limit_attempts: 10,
            rate_limit_window_seconds: 120,
            users: vec!["admin".to_string()],
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
        assert_eq!(parsed.boot_mode, "system");
        assert_eq!(parsed.restart_retries, 5);
        assert_eq!(parsed.restart_delay, 10);
        assert_eq!(parsed.bind_address, "0.0.0.0");
        assert!(parsed.trust_proxy);
        assert_eq!(parsed.rate_limit_attempts, 10);
        assert_eq!(parsed.users, vec!["admin"]);
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

    // Tests for validate_bind_address

    #[test]
    fn test_validate_bind_address_ipv4_localhost() {
        let result = validate_bind_address("127.0.0.1");
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert!(ip.is_loopback());
    }

    #[test]
    fn test_validate_bind_address_ipv4_all_interfaces() {
        let result = validate_bind_address("0.0.0.0");
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert!(ip.is_unspecified());
    }

    #[test]
    fn test_validate_bind_address_ipv6_localhost() {
        let result = validate_bind_address("::1");
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert!(ip.is_loopback());
    }

    #[test]
    fn test_validate_bind_address_ipv6_all_interfaces() {
        let result = validate_bind_address("::");
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert!(ip.is_unspecified());
    }

    #[test]
    fn test_validate_bind_address_localhost_string() {
        let result = validate_bind_address("localhost");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_validate_bind_address_localhost_case_insensitive() {
        let result = validate_bind_address("LOCALHOST");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_validate_bind_address_bracketed_ipv6() {
        let result = validate_bind_address("[::1]");
        assert!(result.is_ok());
        assert!(result.unwrap().is_loopback());
    }

    #[test]
    fn test_validate_bind_address_invalid() {
        let result = validate_bind_address("not-an-ip");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP address"));
    }

    #[test]
    fn test_validate_bind_address_whitespace() {
        let result = validate_bind_address("  127.0.0.1  ");
        assert!(result.is_ok());
    }

    // Tests for is_network_exposed

    #[test]
    fn test_is_network_exposed_ipv4_all() {
        let config = Config {
            bind_address: "0.0.0.0".to_string(),
            ..Config::default()
        };
        assert!(config.is_network_exposed());
    }

    #[test]
    fn test_is_network_exposed_ipv6_all() {
        let config = Config {
            bind_address: "::".to_string(),
            ..Config::default()
        };
        assert!(config.is_network_exposed());
    }

    #[test]
    fn test_is_network_exposed_localhost_false() {
        let config = Config::default();
        assert!(!config.is_network_exposed());
    }

    #[test]
    fn test_is_network_exposed_ipv6_localhost_false() {
        let config = Config {
            bind_address: "::1".to_string(),
            ..Config::default()
        };
        assert!(!config.is_network_exposed());
    }

    // Tests for is_localhost

    #[test]
    fn test_is_localhost_ipv4() {
        let config = Config {
            bind_address: "127.0.0.1".to_string(),
            ..Config::default()
        };
        assert!(config.is_localhost());
    }

    #[test]
    fn test_is_localhost_ipv6() {
        let config = Config {
            bind_address: "::1".to_string(),
            ..Config::default()
        };
        assert!(config.is_localhost());
    }

    #[test]
    fn test_is_localhost_string() {
        let config = Config {
            bind_address: "localhost".to_string(),
            ..Config::default()
        };
        assert!(config.is_localhost());
    }

    #[test]
    fn test_is_localhost_all_interfaces_false() {
        let config = Config {
            bind_address: "0.0.0.0".to_string(),
            ..Config::default()
        };
        assert!(!config.is_localhost());
    }

    // Tests for security fields serialization

    #[test]
    fn test_serialize_deserialize_with_security_fields() {
        let config = Config {
            bind_address: "0.0.0.0".to_string(),
            trust_proxy: true,
            allow_unauthenticated_network: true,
            rate_limit_attempts: 10,
            rate_limit_window_seconds: 120,
            users: vec!["admin".to_string(), "developer".to_string()],
            ..Config::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
        assert_eq!(parsed.bind_address, "0.0.0.0");
        assert!(parsed.trust_proxy);
        assert!(parsed.allow_unauthenticated_network);
        assert_eq!(parsed.rate_limit_attempts, 10);
        assert_eq!(parsed.rate_limit_window_seconds, 120);
        assert_eq!(parsed.users, vec!["admin", "developer"]);
    }
}
