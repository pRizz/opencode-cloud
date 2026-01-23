//! Docker client wrapper with connection handling
//!
//! This module provides a wrapped Docker client that handles connection
//! errors gracefully and provides clear error messages.

use bollard::Docker;
use std::time::Duration;

use super::error::DockerError;
use crate::host::{HostConfig, SshTunnel};

/// Docker client wrapper with connection handling
pub struct DockerClient {
    inner: Docker,
    /// SSH tunnel for remote connections (kept alive for client lifetime)
    _tunnel: Option<SshTunnel>,
    /// Host name for remote connections (None = local)
    host_name: Option<String>,
}

impl DockerClient {
    /// Create new client connecting to local Docker daemon
    ///
    /// Uses platform-appropriate socket (Unix socket on Linux/macOS).
    /// Returns a clear error if Docker is not running or accessible.
    pub fn new() -> Result<Self, DockerError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| DockerError::Connection(e.to_string()))?;

        Ok(Self {
            inner: docker,
            _tunnel: None,
            host_name: None,
        })
    }

    /// Create client with custom timeout (in seconds)
    ///
    /// Use for long-running operations like image builds.
    /// Default timeout is 120 seconds; build timeout should be 600+ seconds.
    pub fn with_timeout(timeout_secs: u64) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| DockerError::Connection(e.to_string()))?
            .with_timeout(Duration::from_secs(timeout_secs));

        Ok(Self {
            inner: docker,
            _tunnel: None,
            host_name: None,
        })
    }

    /// Create client connecting to remote Docker daemon via SSH tunnel
    ///
    /// Establishes an SSH tunnel to the remote host and connects Bollard
    /// to the forwarded local port.
    ///
    /// # Arguments
    /// * `host` - Remote host configuration
    /// * `host_name` - Name of the host (for display purposes)
    pub async fn connect_remote(host: &HostConfig, host_name: &str) -> Result<Self, DockerError> {
        // Create SSH tunnel
        let tunnel = SshTunnel::new(host, host_name)
            .map_err(|e| DockerError::Connection(format!("SSH tunnel failed: {}", e)))?;

        // Wait for tunnel to be ready with exponential backoff
        tunnel
            .wait_ready()
            .await
            .map_err(|e| DockerError::Connection(format!("SSH tunnel not ready: {}", e)))?;

        // Connect Bollard to the tunnel's local port
        let docker_url = tunnel.docker_url();
        tracing::debug!("Connecting to remote Docker via {}", docker_url);

        // Retry connection with backoff (tunnel may need a moment)
        let max_attempts = 3;
        let mut last_err = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                tracing::debug!("Retry attempt {} after {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            match Docker::connect_with_http(&docker_url, 120, bollard::API_DEFAULT_VERSION) {
                Ok(docker) => {
                    // Verify connection works
                    match docker.ping().await {
                        Ok(_) => {
                            tracing::info!("Connected to Docker on {} via SSH tunnel", host_name);
                            return Ok(Self {
                                inner: docker,
                                _tunnel: Some(tunnel),
                                host_name: Some(host_name.to_string()),
                            });
                        }
                        Err(e) => {
                            tracing::debug!("Ping failed: {}", e);
                            last_err = Some(e.to_string());
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Connection failed: {}", e);
                    last_err = Some(e.to_string());
                }
            }
        }

        Err(DockerError::Connection(format!(
            "Failed to connect to Docker on {}: {}",
            host_name,
            last_err.unwrap_or_else(|| "unknown error".to_string())
        )))
    }

    /// Create remote client with custom timeout
    pub async fn connect_remote_with_timeout(
        host: &HostConfig,
        host_name: &str,
        timeout_secs: u64,
    ) -> Result<Self, DockerError> {
        let tunnel = SshTunnel::new(host, host_name)
            .map_err(|e| DockerError::Connection(format!("SSH tunnel failed: {}", e)))?;

        tunnel
            .wait_ready()
            .await
            .map_err(|e| DockerError::Connection(format!("SSH tunnel not ready: {}", e)))?;

        let docker_url = tunnel.docker_url();

        let docker =
            Docker::connect_with_http(&docker_url, timeout_secs, bollard::API_DEFAULT_VERSION)
                .map_err(|e| DockerError::Connection(e.to_string()))?;

        // Verify connection
        docker.ping().await.map_err(DockerError::from)?;

        Ok(Self {
            inner: docker,
            _tunnel: Some(tunnel),
            host_name: Some(host_name.to_string()),
        })
    }

    /// Verify connection to Docker daemon
    ///
    /// Returns Ok(()) if connected, descriptive error otherwise.
    pub async fn verify_connection(&self) -> Result<(), DockerError> {
        self.inner.ping().await.map_err(DockerError::from)?;
        Ok(())
    }

    /// Get Docker version info (useful for debugging)
    pub async fn version(&self) -> Result<String, DockerError> {
        let version = self.inner.version().await.map_err(DockerError::from)?;

        let version_str = format!(
            "Docker {} (API {})",
            version.version.unwrap_or_else(|| "unknown".to_string()),
            version.api_version.unwrap_or_else(|| "unknown".to_string())
        );

        Ok(version_str)
    }

    /// Get the host name if this is a remote connection
    pub fn host_name(&self) -> Option<&str> {
        self.host_name.as_deref()
    }

    /// Check if this is a remote connection
    pub fn is_remote(&self) -> bool {
        self._tunnel.is_some()
    }

    /// Access inner Bollard client for advanced operations
    pub fn inner(&self) -> &Docker {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_client_creation_does_not_panic() {
        // This test just verifies the code compiles and doesn't panic
        // Actual connection test requires Docker to be running
        let result = DockerClient::new();
        // We don't assert success because Docker may not be running in CI
        drop(result);
    }

    #[test]
    fn docker_client_with_timeout_does_not_panic() {
        let result = DockerClient::with_timeout(600);
        drop(result);
    }

    #[test]
    fn test_host_name_methods() {
        // Local client has no host name
        if let Ok(client) = DockerClient::new() {
            assert!(client.host_name().is_none());
            assert!(!client.is_remote());
        }
    }
}
