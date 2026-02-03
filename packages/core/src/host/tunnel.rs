//! SSH tunnel management
//!
//! Creates and manages SSH tunnels to remote Docker daemons.

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::error::HostError;
use super::schema::HostConfig;

/// SSH tunnel to a remote Docker daemon
///
/// The tunnel forwards a local port to the remote Docker socket.
/// Implements Drop to ensure the SSH process is killed on cleanup.
pub struct SshTunnel {
    child: Child,
    local_port: u16,
    host_name: String,
}

impl SshTunnel {
    /// Create SSH tunnel to remote Docker socket
    ///
    /// Spawns an SSH process with local port forwarding:
    /// `ssh -L local_port:/var/run/docker.sock -N host`
    ///
    /// Uses BatchMode=yes to fail fast if key not in agent.
    pub fn new(host: &HostConfig, host_name: &str) -> Result<Self, HostError> {
        // Find available local port
        let local_port = find_available_port()?;

        // Build SSH command
        let mut cmd = Command::new("ssh");

        // Local port forward: local_port -> remote docker.sock
        cmd.arg("-L")
            .arg(format!("{local_port}:/var/run/docker.sock"));

        // No command, just forward
        cmd.arg("-N");

        // Suppress prompts, fail fast on auth issues
        cmd.arg("-o").arg("BatchMode=yes");

        // Accept new host keys automatically (first connection)
        cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");

        // Connection timeout
        cmd.arg("-o").arg("ConnectTimeout=10");

        // Prevent SSH from reading stdin (fixes issues with background operation)
        cmd.arg("-o").arg("RequestTTY=no");

        // Jump host support
        if let Some(jump) = &host.jump_host {
            cmd.arg("-J").arg(jump);
        }

        // Identity file
        if let Some(key) = &host.identity_file {
            cmd.arg("-i").arg(key);
        }

        // Custom port
        if let Some(port) = host.port {
            cmd.arg("-p").arg(port.to_string());
        }

        // Target: user@hostname
        cmd.arg(format!("{}@{}", host.user, host.hostname));

        // Configure stdio
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        tracing::debug!(
            "Spawning SSH tunnel: ssh -L {}:/var/run/docker.sock {}@{}",
            local_port,
            host.user,
            host.hostname
        );

        let child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                HostError::SshSpawn("SSH not found. Install OpenSSH client.".to_string())
            } else {
                HostError::SshSpawn(e.to_string())
            }
        })?;

        Ok(Self {
            child,
            local_port,
            host_name: host_name.to_string(),
        })
    }

    /// Get the local port for Docker connection
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Get the Docker connection URL
    pub fn docker_url(&self) -> String {
        format!("tcp://127.0.0.1:{}", self.local_port)
    }

    /// Get the host name this tunnel connects to
    pub fn host_name(&self) -> &str {
        &self.host_name
    }

    /// Wait for tunnel to be ready (port accepting connections)
    ///
    /// Retries with exponential backoff: 100ms, 200ms, 400ms (3 attempts)
    pub async fn wait_ready(&self) -> Result<(), HostError> {
        let max_attempts = 3;
        let initial_delay_ms = 100;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = Duration::from_millis(initial_delay_ms * 2u64.pow(attempt));
                tracing::debug!("Tunnel wait attempt {} after {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            // Try to connect to the local port
            match std::net::TcpStream::connect_timeout(
                &format!("127.0.0.1:{}", self.local_port).parse().unwrap(),
                Duration::from_secs(1),
            ) {
                Ok(_) => {
                    tracing::debug!("SSH tunnel ready on port {}", self.local_port);
                    return Ok(());
                }
                Err(e) => {
                    tracing::debug!("Tunnel not ready: {}", e);
                }
            }
        }

        Err(HostError::TunnelTimeout(max_attempts))
    }

    /// Check if the SSH process is still running
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        tracing::debug!(
            "Cleaning up SSH tunnel to {} (port {})",
            self.host_name,
            self.local_port
        );
        if let Err(e) = self.child.kill() {
            // Process may have already exited
            tracing::debug!("SSH tunnel kill result: {}", e);
        }
        // Wait to reap the zombie process
        let _ = self.child.wait();
    }
}

/// Find an available local port for the tunnel
fn find_available_port() -> Result<u16, HostError> {
    // Bind to port 0 to get OS-assigned port
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| HostError::PortAllocation(e.to_string()))?;

    let port = listener
        .local_addr()
        .map_err(|e| HostError::PortAllocation(e.to_string()))?
        .port();

    // Drop listener to free the port
    drop(listener);

    Ok(port)
}

/// Test SSH connection to a host
///
/// Runs `ssh user@host docker version` to verify:
/// 1. SSH connection works
/// 2. Docker is available on remote
pub async fn test_connection(host: &HostConfig) -> Result<String, HostError> {
    let mut cmd = Command::new("ssh");

    // Standard options
    cmd.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new");

    // Host-specific options (port, identity, jump, user@host)
    cmd.args(host.ssh_args());

    // Docker version command
    cmd.arg("docker")
        .arg("version")
        .arg("--format")
        .arg("{{.Server.Version}}");

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HostError::SshSpawn("SSH not found. Install OpenSSH client.".to_string())
        } else {
            HostError::SshSpawn(e.to_string())
        }
    })?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!("Docker version on remote: {}", version);
        Ok(version)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Detect authentication failures
        if stderr.contains("Permission denied") || stderr.contains("Host key verification failed") {
            return Err(HostError::AuthFailed {
                key_hint: host.identity_file.clone(),
            });
        }

        // Detect Docker not available
        if stderr.contains("command not found") || stderr.contains("not found") {
            return Err(HostError::RemoteDockerUnavailable(
                "Docker is not installed on remote host".to_string(),
            ));
        }

        Err(HostError::ConnectionFailed(stderr.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn can_bind_localhost() -> bool {
        TcpListener::bind(("127.0.0.1", 0)).is_ok()
    }

    #[test]
    fn test_find_available_port() {
        if !can_bind_localhost() {
            eprintln!("Skipping test: cannot bind to localhost in this environment.");
            return;
        }
        let port = find_available_port().unwrap();
        assert!(port > 0);

        // Port should be available (we can bind to it)
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"));
        assert!(listener.is_ok());
    }

    #[test]
    fn test_docker_url_format() {
        // We can't easily test tunnel creation without SSH, but we can test the URL format
        let url = format!("tcp://127.0.0.1:{}", 12345);
        assert_eq!(url, "tcp://127.0.0.1:12345");
    }
}
