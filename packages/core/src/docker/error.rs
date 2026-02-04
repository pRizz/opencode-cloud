//! Docker-specific error types
//!
//! This module defines errors that can occur during Docker operations,
//! providing clear, actionable messages for common issues.

use thiserror::Error;

/// Errors that can occur during Docker operations
#[derive(Error, Debug)]
pub enum DockerError {
    /// Failed to connect to the Docker daemon
    #[error("Docker connection failed: {0}")]
    Connection(String),

    /// Docker daemon is not running
    #[error("Docker daemon not running. Start Docker Desktop or the Docker service.")]
    NotRunning,

    /// Docker socket not found
    #[error("Docker socket not found. Is Docker installed and running?")]
    SocketNotFound,

    /// Permission denied accessing Docker socket
    #[error(
        "Permission denied accessing Docker socket. You may need to add your user to the 'docker' group."
    )]
    PermissionDenied,

    /// Failed to build Docker image
    #[error("Docker build failed: {0}")]
    Build(String),

    /// Failed to pull Docker image
    #[error("Docker pull failed: {0}")]
    Pull(String),

    /// Image operation failed
    #[error("Docker image operation failed: {0}")]
    Image(String),

    /// Container operation failed
    #[error("Container operation failed: {0}")]
    Container(String),

    /// Volume operation failed
    #[error("Volume operation failed: {0}")]
    Volume(String),

    /// Operation timed out
    #[error("Docker operation timed out")]
    Timeout,
}

impl From<bollard::errors::Error> for DockerError {
    fn from(err: bollard::errors::Error) -> Self {
        let msg = err.to_string();

        // Detect common error patterns and provide better messages
        if msg.contains("No such file or directory") {
            DockerError::SocketNotFound
        } else if msg.contains("Cannot connect to the Docker daemon")
            || msg.contains("connection refused")
        {
            DockerError::NotRunning
        } else if msg.contains("permission denied") || msg.contains("Permission denied") {
            DockerError::PermissionDenied
        } else {
            DockerError::Connection(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_error_displays_correctly() {
        let err = DockerError::NotRunning;
        assert!(err.to_string().contains("Docker daemon not running"));

        let err = DockerError::SocketNotFound;
        assert!(err.to_string().contains("Docker socket not found"));

        let err = DockerError::PermissionDenied;
        assert!(err.to_string().contains("Permission denied"));

        let err = DockerError::Build("layer failed".to_string());
        assert!(err.to_string().contains("layer failed"));
    }
}
