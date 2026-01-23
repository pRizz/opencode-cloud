//! Host-specific error types
//!
//! Errors that can occur during remote host operations.

use thiserror::Error;

/// Errors that can occur during host operations
#[derive(Error, Debug)]
pub enum HostError {
    /// Failed to spawn SSH process
    #[error("Failed to spawn SSH: {0}")]
    SshSpawn(String),

    /// SSH connection failed
    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),

    /// SSH authentication failed (key not in agent, passphrase needed)
    #[error("SSH authentication failed. Ensure your key is loaded: ssh-add {}", .key_hint.as_deref().unwrap_or("~/.ssh/id_rsa"))]
    AuthFailed { key_hint: Option<String> },

    /// Host not found in hosts.json
    #[error("Host not found: {0}")]
    NotFound(String),

    /// Host already exists
    #[error("Host already exists: {0}")]
    AlreadyExists(String),

    /// Failed to allocate local port for tunnel
    #[error("Failed to allocate local port: {0}")]
    PortAllocation(String),

    /// Failed to load hosts file
    #[error("Failed to load hosts file: {0}")]
    LoadFailed(String),

    /// Failed to save hosts file
    #[error("Failed to save hosts file: {0}")]
    SaveFailed(String),

    /// Invalid host configuration
    #[error("Invalid host configuration: {0}")]
    InvalidConfig(String),

    /// Tunnel connection timed out
    #[error("SSH tunnel connection timed out after {0} attempts")]
    TunnelTimeout(u32),

    /// Remote Docker not available
    #[error("Docker not available on remote host: {0}")]
    RemoteDockerUnavailable(String),
}
