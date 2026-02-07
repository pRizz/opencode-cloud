//! URL formatting utilities for CLI output
//!
//! This module provides centralized URL formatting helpers to ensure
//! consistent URL display across all CLI commands.
//!
//! Note: These functions are provided for future integration but not yet
//! used by command implementations. The allow(dead_code) permits this
//! until commands are refactored to use these shared utilities.

#![allow(dead_code)]

use opencode_cloud_core::load_hosts;

/// Resolve the remote address for a host by looking up its configuration.
///
/// Returns the hostname from the host configuration, or None if:
/// - No host name is provided
/// - The hosts configuration cannot be loaded
/// - The host is not found in configuration
///
/// # Arguments
///
/// * `host_name` - Optional name of the configured remote host
///
/// # Returns
///
/// The remote hostname string if found, None otherwise
pub fn resolve_remote_addr(host_name: Option<&str>) -> Option<String> {
    host_name.and_then(|name| {
        load_hosts()
            .ok()
            .and_then(|h| h.get_host(name).map(|cfg| cfg.hostname.clone()))
    })
}

/// Normalize a bind address for browser/display use.
///
/// When the bind address is a wildcard (0.0.0.0 or ::), this returns
/// 127.0.0.1 for local access. Otherwise returns the original address.
///
/// # Arguments
///
/// * `bind_addr` - The configured bind address
///
/// # Returns
///
/// A display-friendly address string
pub fn normalize_bind_addr(bind_addr: &str) -> &str {
    if bind_addr == "0.0.0.0" || bind_addr == "::" {
        "127.0.0.1"
    } else {
        bind_addr
    }
}

/// Normalize a bind address for user-facing URL display/opening.
///
/// For loopback and wildcard bind addresses, this returns `localhost` so browser
/// navigation uses passkey-eligible localhost origins.
pub fn localhost_display_addr(bind_addr: &str) -> &str {
    if bind_addr.eq_ignore_ascii_case("localhost")
        || matches!(bind_addr, "127.0.0.1" | "::1" | "0.0.0.0" | "::")
    {
        "localhost"
    } else {
        bind_addr
    }
}

/// Format a Cockpit URL for display.
///
/// Uses the remote address if available, otherwise normalizes the bind
/// address for localhost-first local display.
///
/// # Arguments
///
/// * `maybe_remote_addr` - Optional remote hostname (from resolve_remote_addr)
/// * `bind_addr` - The configured bind address
/// * `cockpit_port` - The configured Cockpit port
///
/// # Returns
///
/// A formatted Cockpit URL string
pub fn format_cockpit_url(
    maybe_remote_addr: Option<&str>,
    bind_addr: &str,
    cockpit_port: u16,
) -> String {
    if let Some(remote_addr) = maybe_remote_addr {
        format!("http://{remote_addr}:{cockpit_port}")
    } else {
        let cockpit_addr = localhost_display_addr(bind_addr);
        format!("http://{cockpit_addr}:{cockpit_port}")
    }
}

/// Format a service URL for display.
///
/// Uses the remote address if available, otherwise uses the bind address
/// normalized for localhost-first local display.
///
/// # Arguments
///
/// * `maybe_remote_addr` - Optional remote hostname (from resolve_remote_addr)
/// * `bind_addr` - The configured bind address
/// * `port` - The service port
///
/// # Returns
///
/// A formatted service URL string
pub fn format_service_url(maybe_remote_addr: Option<&str>, bind_addr: &str, port: u16) -> String {
    if let Some(remote_addr) = maybe_remote_addr {
        format!("http://{remote_addr}:{port}")
    } else {
        let service_addr = localhost_display_addr(bind_addr);
        format!("http://{service_addr}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_bind_addr_normalizes_ipv4_wildcard() {
        assert_eq!(normalize_bind_addr("0.0.0.0"), "127.0.0.1");
    }

    #[test]
    fn normalize_bind_addr_normalizes_ipv6_wildcard() {
        assert_eq!(normalize_bind_addr("::"), "127.0.0.1");
    }

    #[test]
    fn normalize_bind_addr_preserves_localhost() {
        assert_eq!(normalize_bind_addr("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn normalize_bind_addr_preserves_specific_ip() {
        assert_eq!(normalize_bind_addr("192.168.1.100"), "192.168.1.100");
    }

    #[test]
    fn format_cockpit_url_uses_remote_addr_when_present() {
        let url = format_cockpit_url(Some("myserver.local"), "127.0.0.1", 9090);
        assert_eq!(url, "http://myserver.local:9090");
    }

    #[test]
    fn format_cockpit_url_normalizes_wildcard_address() {
        let url = format_cockpit_url(None, "0.0.0.0", 9090);
        assert_eq!(url, "http://localhost:9090");
    }

    #[test]
    fn format_cockpit_url_preserves_specific_address() {
        let url = format_cockpit_url(None, "192.168.1.100", 9090);
        assert_eq!(url, "http://192.168.1.100:9090");
    }

    #[test]
    fn format_service_url_uses_remote_addr_when_present() {
        let url = format_service_url(Some("myserver.local"), "127.0.0.1", 3000);
        assert_eq!(url, "http://myserver.local:3000");
    }

    #[test]
    fn format_service_url_uses_bind_addr_when_no_remote() {
        let url = format_service_url(None, "0.0.0.0", 3000);
        assert_eq!(url, "http://localhost:3000");
    }

    #[test]
    fn localhost_display_addr_normalizes_loopback_and_wildcard() {
        assert_eq!(localhost_display_addr("127.0.0.1"), "localhost");
        assert_eq!(localhost_display_addr("::1"), "localhost");
        assert_eq!(localhost_display_addr("0.0.0.0"), "localhost");
        assert_eq!(localhost_display_addr("::"), "localhost");
    }

    #[test]
    fn localhost_display_addr_preserves_non_loopback_address() {
        assert_eq!(localhost_display_addr("192.168.1.100"), "192.168.1.100");
    }

    #[test]
    fn resolve_remote_addr_returns_none_for_none_host() {
        let result = resolve_remote_addr(None);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_remote_addr_returns_none_for_unknown_host() {
        // Host "nonexistent_test_host_12345" won't exist in any real config
        let result = resolve_remote_addr(Some("nonexistent_test_host_12345"));
        assert!(result.is_none());
    }
}
