//! Health check module for OpenCode service
//!
//! Provides health checking functionality by querying OpenCode's /global/health endpoint.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

use super::DockerClient;

/// Response from OpenCode's /global/health endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Service version string
    pub version: String,
}

/// Extended health response including container stats
#[derive(Debug, Serialize)]
pub struct ExtendedHealthResponse {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Service version string
    pub version: String,
    /// Container state (running, stopped, etc.)
    pub container_state: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage in megabytes (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage_mb: Option<u64>,
}

/// Errors that can occur during health checks
#[derive(Debug, Error)]
pub enum HealthError {
    /// HTTP request failed
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    /// Service returned non-200 status
    #[error("Service unhealthy (HTTP {0})")]
    Unhealthy(u16),

    /// Connection refused - service may not be running
    #[error("Connection refused - service may not be running")]
    ConnectionRefused,

    /// Request timed out - service may be starting
    #[error("Timeout - service may be starting")]
    Timeout,
}

fn format_host(bind_addr: &str) -> String {
    if bind_addr.contains(':') && !bind_addr.starts_with('[') {
        format!("[{bind_addr}]")
    } else {
        bind_addr.to_string()
    }
}

/// Check health by querying OpenCode's /global/health endpoint
///
/// Returns the health response on success (HTTP 200).
/// Returns an error for connection issues, timeouts, or non-200 responses.
pub async fn check_health(bind_addr: &str, port: u16) -> Result<HealthResponse, HealthError> {
    let host = format_host(bind_addr);
    let url = format!("http://{host}:{port}/global/health");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            // Check for connection refused
            if e.is_connect() {
                return Err(HealthError::ConnectionRefused);
            }
            // Check for timeout
            if e.is_timeout() {
                return Err(HealthError::Timeout);
            }
            return Err(HealthError::RequestError(e));
        }
    };

    let status = response.status();

    if status.is_success() {
        let health_response = response.json::<HealthResponse>().await?;
        Ok(health_response)
    } else {
        Err(HealthError::Unhealthy(status.as_u16()))
    }
}

/// Check health with extended information including container stats
///
/// Combines basic health check with container statistics from Docker.
/// If container stats fail, still returns response with container_state = "unknown".
pub async fn check_health_extended(
    client: &DockerClient,
    bind_addr: &str,
    port: u16,
) -> Result<ExtendedHealthResponse, HealthError> {
    // Get basic health info
    let health = check_health(bind_addr, port).await?;

    // Get container stats
    let container_name = super::CONTAINER_NAME;

    // Try to get container info
    let (container_state, uptime_seconds, memory_usage_mb) =
        match client.inner().inspect_container(container_name, None).await {
            Ok(info) => {
                let state = info
                    .state
                    .as_ref()
                    .and_then(|s| s.status.as_ref())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // Calculate uptime
                let uptime = info
                    .state
                    .as_ref()
                    .and_then(|s| s.started_at.as_ref())
                    .and_then(|started| {
                        let timestamp = chrono::DateTime::parse_from_rfc3339(started).ok()?;
                        let now = chrono::Utc::now();
                        let started_utc = timestamp.with_timezone(&chrono::Utc);
                        if now >= started_utc {
                            Some((now - started_utc).num_seconds() as u64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);

                // Get memory usage (would require stats API call - skip for now)
                let memory = None;

                (state, uptime, memory)
            }
            Err(_) => ("unknown".to_string(), 0, None),
        };

    Ok(ExtendedHealthResponse {
        healthy: health.healthy,
        version: health.version,
        container_state,
        uptime_seconds,
        memory_usage_mb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_connection_refused() {
        // Port 1 should always refuse connection
        let result = check_health("127.0.0.1", 1).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            HealthError::ConnectionRefused => {}
            other => panic!("Expected ConnectionRefused, got: {other:?}"),
        }
    }

    #[test]
    fn format_host_wraps_ipv6() {
        assert_eq!(format_host("::1"), "[::1]");
    }

    #[test]
    fn format_host_preserves_ipv4() {
        assert_eq!(format_host("127.0.0.1"), "127.0.0.1");
    }
}
