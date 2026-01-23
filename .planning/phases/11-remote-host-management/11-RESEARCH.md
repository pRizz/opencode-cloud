# Phase 11: Remote Host Management - Research

**Researched:** 2026-01-23
**Domain:** SSH tunneling, remote Docker management, CLI host routing
**Confidence:** HIGH

## Summary

This research covers how to implement remote Docker host management via SSH tunnels. The decision to use SSH (not Docker TLS API) is locked from CONTEXT.md, simplifying the architecture - we tunnel all Docker traffic through SSH rather than exposing Docker ports.

The recommended approach is to **shell out to system `ssh`** rather than use native Rust SSH libraries. This provides the best compatibility with existing SSH infrastructure (jump hosts, SSH configs, SSH agents) without reimplementing complex SSH features. Bollard's built-in SSH feature requires the system `ssh` command anyway, making this the natural fit.

**Primary recommendation:** Use `std::process::Command` to spawn `ssh -L` for local port forwarding, then connect Bollard to `tcp://127.0.0.1:<forwarded_port>`. Store hosts in a separate `hosts.json` file to avoid polluting the main config. Add a `--host` global flag to all container commands.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| System SSH | (system) | SSH tunnel creation | Full SSH config support, agent, jump hosts |
| bollard | 0.18 | Docker API client | Already in use, supports HTTP/TCP connections |
| tokio::process | (tokio) | Async process spawning | Manage SSH tunnel processes |
| dialoguer | 0.11 | Interactive prompts | Already in use for wizards |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rpassword | 1.0 | Secure password input | SSH key passphrase prompts |
| whoami | 1.5 | Current username | Default SSH username |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| System ssh | russh | Native Rust but lacks SSH config parsing, agent integration complexity |
| System ssh | ssh2-rs | C dependency (libssh2), sync-only, no jump host support |
| System ssh | bollard ssh feature | Uses system ssh anyway, less control over tunnel lifecycle |
| hosts.json | Embed in config.json | Cleaner separation, avoids schema migration complexity |

**Installation:**
```bash
# No new crates needed - use existing tokio + bollard
# Optional for better UX:
cargo add rpassword whoami
```

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── host/                    # NEW: Host management module
│   ├── mod.rs              # Public exports
│   ├── schema.rs           # HostConfig, HostsFile structs
│   ├── storage.rs          # Load/save hosts.json
│   ├── tunnel.rs           # SSH tunnel management
│   └── error.rs            # Host-specific errors
├── docker/
│   └── client.rs           # Extend with remote connection support
└── config/
    └── paths.rs            # Add get_hosts_path()

packages/cli-rust/src/
├── commands/
│   ├── host/               # NEW: Host management commands
│   │   ├── mod.rs
│   │   ├── add.rs          # occ host add
│   │   ├── remove.rs       # occ host remove
│   │   ├── list.rs         # occ host list
│   │   ├── show.rs         # occ host show
│   │   ├── edit.rs         # occ host edit
│   │   ├── test.rs         # occ host test
│   │   └── default.rs      # occ host default
│   └── mod.rs              # Add Host variant to Commands
└── lib.rs                  # Add --host global flag
```

### Pattern 1: SSH Tunnel as Background Process
**What:** Spawn SSH in background, monitor process, clean up on command completion
**When to use:** Every remote Docker operation
**Example:**
```rust
// Source: std::process::Command + tokio patterns
use std::process::{Child, Command, Stdio};
use std::net::TcpListener;

pub struct SshTunnel {
    child: Child,
    local_port: u16,
}

impl SshTunnel {
    /// Create SSH tunnel to remote Docker socket
    pub fn new(host: &HostConfig) -> Result<Self, HostError> {
        // Find available local port
        let local_port = find_available_port()?;

        // Build SSH command with all options
        let mut cmd = Command::new("ssh");

        // Local port forward: local_port -> remote docker.sock
        cmd.arg("-L")
           .arg(format!("{}:/var/run/docker.sock", local_port));

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

        // Target: user@host with "sleep infinity" to keep tunnel open
        cmd.arg(format!("{}@{}", host.user, host.hostname))
           .arg("-N");  // No command, just forward

        // Suppress prompts, fail fast
        cmd.arg("-o").arg("BatchMode=yes")
           .arg("-o").arg("StrictHostKeyChecking=accept-new")
           .arg("-o").arg("ConnectTimeout=10");

        cmd.stdin(Stdio::null())
           .stdout(Stdio::null())
           .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| HostError::SshSpawn(e.to_string()))?;

        Ok(Self { child, local_port })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn find_available_port() -> Result<u16, HostError> {
    // Bind to port 0 to get OS-assigned port
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| HostError::PortAllocation(e.to_string()))?;
    Ok(listener.local_addr()?.port())
}
```

### Pattern 2: Extended DockerClient with Remote Support
**What:** DockerClient gains a `connect_remote()` method
**When to use:** When --host flag is present
**Example:**
```rust
// Source: bollard docs + custom tunnel integration
impl DockerClient {
    /// Create client connecting to remote Docker via SSH tunnel
    pub async fn connect_remote(host: &HostConfig) -> Result<Self, DockerError> {
        // Create tunnel
        let tunnel = SshTunnel::new(host)
            .map_err(|e| DockerError::Connection(e.to_string()))?;

        // Wait for tunnel to be ready (retry connection)
        let local_addr = format!("tcp://127.0.0.1:{}", tunnel.local_port());

        // Retry loop with exponential backoff
        let max_attempts = 3;
        let mut last_err = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt as u32));
                tokio::time::sleep(delay).await;
            }

            match Docker::connect_with_http(&local_addr, 120, API_DEFAULT_VERSION) {
                Ok(docker) => {
                    // Verify connection works
                    if docker.ping().await.is_ok() {
                        return Ok(Self {
                            inner: docker,
                            _tunnel: Some(tunnel),  // Keep tunnel alive
                        });
                    }
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(DockerError::Connection(
            last_err.map(|e| e.to_string())
                .unwrap_or_else(|| "Tunnel connection failed".to_string())
        ))
    }
}
```

### Pattern 3: Global --host Flag with Routing
**What:** Add --host to all container commands, resolve to DockerClient
**When to use:** CLI command dispatch
**Example:**
```rust
// In packages/cli-rust/src/lib.rs
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Target host (default: local, or default_host from config)
    #[arg(long, global = true)]
    host: Option<String>,

    // ... existing fields
}

// Helper to get DockerClient based on --host flag
async fn get_docker_client(
    maybe_host: Option<&str>,
    hosts: &HostsFile,
) -> Result<(DockerClient, Option<String>)> {
    let host_name = maybe_host
        .map(String::from)
        .or_else(|| hosts.default_host.clone());

    match host_name {
        Some(name) if name != "local" => {
            let host = hosts.hosts.get(&name)
                .ok_or_else(|| anyhow!("Unknown host: {}", name))?;
            let client = DockerClient::connect_remote(host).await?;
            Ok((client, Some(name)))
        }
        _ => {
            let client = DockerClient::new()?;
            Ok((client, None))
        }
    }
}

// Usage in commands - prefix output with host name
async fn cmd_start(args: &StartArgs, host_name: Option<&str>, ...) -> Result<()> {
    if let Some(name) = host_name {
        println!("[{}] Starting container...", style(name).cyan());
    }
    // ... rest of command
}
```

### Anti-Patterns to Avoid
- **Hand-rolling SSH protocol:** Don't implement SSH yourself. System ssh handles edge cases (host key verification, agent forwarding, config parsing).
- **Blocking on tunnel creation:** Always use async/timeout patterns. SSH connections can hang indefinitely.
- **Storing passwords:** Never store SSH passwords. Require key-based auth only.
- **Global tunnel singleton:** Each command should create its own tunnel. Persistent tunnels add complexity and stale connection issues.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SSH key passphrase prompt | Custom stdin handling | `rpassword::prompt_password()` | Cross-platform, handles terminal modes |
| SSH connection | Pure Rust SSH client | System `ssh` command | SSH config, agent, jump hosts all work automatically |
| Available port detection | Manual port scanning | `TcpListener::bind("127.0.0.1:0")` | OS handles race conditions |
| SSH config parsing | Config file parser | System `ssh` | `ssh` reads `~/.ssh/config` automatically |
| Host key verification | Manual known_hosts | `-o StrictHostKeyChecking=accept-new` | Safe default, adds new hosts automatically |

**Key insight:** SSH is a complex protocol with decades of edge cases. Shelling out to `ssh` gives you all the battle-tested behavior for free.

## Common Pitfalls

### Pitfall 1: Tunnel Process Leaks
**What goes wrong:** SSH tunnel process outlives the command, accumulates zombie processes
**Why it happens:** Forgot to kill child process on Drop, or panic before cleanup
**How to avoid:**
- Implement `Drop` for tunnel struct that kills the child process
- Use `tokio::select!` with timeout for operations
- Store tunnel in DockerClient so it lives exactly as long as needed
**Warning signs:** Stale SSH processes in `ps aux | grep ssh`

### Pitfall 2: Connection Refused After Tunnel Starts
**What goes wrong:** Bollard connection fails even though SSH tunnel started
**Why it happens:** SSH tunnel takes time to establish; connecting immediately fails
**How to avoid:**
- Retry loop with exponential backoff (100ms, 200ms, 400ms)
- Wait for local port to accept connections before returning
- Maximum 3 retries as specified in CONTEXT.md
**Warning signs:** Intermittent "connection refused" errors

### Pitfall 3: Jump Host Auth Failure
**What goes wrong:** SSH works locally but fails with jump host
**Why it happens:** Jump host and target host need different credentials
**How to avoid:**
- Support separate identity_file for jump host via SSH config
- Document that users should configure `~/.ssh/config` for complex setups
- Test connection with `occ host test` before using
**Warning signs:** "Permission denied (publickey)" after successful jump

### Pitfall 4: Output Interleaving on Multiple Hosts
**What goes wrong:** Output from different hosts gets mixed up (future multi-host)
**Why it happens:** Concurrent async operations writing to stdout
**How to avoid:**
- Prefix ALL output with `[hostname]` as specified in CONTEXT.md
- Use line-buffered output
- For Phase 11, single host per command avoids this entirely
**Warning signs:** Confusing output that doesn't indicate source

### Pitfall 5: Encrypted Key Passphrase Deadlock
**What goes wrong:** Command hangs waiting for passphrase that can't be entered
**Why it happens:** SSH in BatchMode can't prompt for passphrase; ssh-agent not running
**How to avoid:**
- Default to `BatchMode=yes` to fail fast rather than hang
- Pre-flight check: test SSH connection with timeout
- Document: "Add keys to ssh-agent before using"
- Clear error message: "SSH key requires passphrase. Run `ssh-add` first."
**Warning signs:** Command hangs indefinitely with no output

## Code Examples

Verified patterns from official sources:

### Host Configuration Schema
```rust
// Source: Custom design based on SSH config options + CONTEXT.md requirements
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a remote host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// SSH hostname or IP address
    pub hostname: String,

    /// SSH username (default: current user)
    #[serde(default)]
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

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            user: whoami::username(),
            port: None,
            identity_file: None,
            jump_host: None,
            groups: Vec::new(),
            description: None,
        }
    }
}

/// Root structure for hosts.json file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HostsFile {
    /// Version for future migrations
    pub version: u32,

    /// Default host name (None = local Docker)
    #[serde(default)]
    pub default_host: Option<String>,

    /// Map of host name to configuration
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
}
```

### SSH Tunnel Test Connection
```rust
// Source: Pattern from occ host test command
use std::process::{Command, Stdio};
use std::time::Duration;

/// Test SSH connection to a host
pub async fn test_ssh_connection(host: &HostConfig) -> Result<(), HostError> {
    let mut cmd = Command::new("ssh");

    // Standard options
    cmd.arg("-o").arg("BatchMode=yes")
       .arg("-o").arg("ConnectTimeout=10")
       .arg("-o").arg("StrictHostKeyChecking=accept-new");

    // Host-specific options
    if let Some(port) = host.port {
        cmd.arg("-p").arg(port.to_string());
    }
    if let Some(key) = &host.identity_file {
        cmd.arg("-i").arg(key);
    }
    if let Some(jump) = &host.jump_host {
        cmd.arg("-J").arg(jump);
    }

    // Target with simple command
    cmd.arg(format!("{}@{}", host.user, host.hostname))
       .arg("docker version --format '{{.Server.Version}}'");

    cmd.stdin(Stdio::null())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    let output = cmd.output()
        .map_err(|e| HostError::SshSpawn(e.to_string()))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Docker version on remote: {}", version.trim());
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(HostError::ConnectionFailed(stderr.to_string()))
    }
}
```

### Exponential Backoff Retry
```rust
// Source: Existing pattern in codebase + CONTEXT.md (3 retries)
use std::time::Duration;

/// Retry with exponential backoff
/// Initial delay: 100ms, factor: 2x, max attempts: 3
pub async fn retry_with_backoff<F, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
{
    let max_attempts = 3;
    let initial_delay_ms = 100;
    let mut last_err = None;

    for attempt in 0..max_attempts {
        if attempt > 0 {
            let delay = Duration::from_millis(initial_delay_ms * 2u64.pow(attempt as u32));
            tracing::debug!("Retry attempt {} after {:?}", attempt + 1, delay);
            tokio::time::sleep(delay).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => last_err = Some(e),
        }
    }

    Err(last_err.expect("At least one attempt must have run"))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Docker TLS over TCP | Docker over SSH | Docker 18.09 (2018) | No need to expose Docker daemon, uses existing SSH |
| Manual SSH config | `docker context` | Docker 19.03 (2019) | Native Docker CLI support for SSH contexts |
| Pure Rust SSH (thrussh) | russh | 2022 | Modern async SSH, actively maintained |

**Deprecated/outdated:**
- **ssh2-rs for async:** Library is synchronous; requires wrapping in spawn_blocking
- **Direct Docker TCP:** Security risk; SSH tunnel preferred for remote access
- **Manual port forwarding setup:** `docker context` handles this automatically for Docker CLI

## Open Questions

Things that couldn't be fully resolved:

1. **Bollard SSH feature vs manual tunnel**
   - What we know: Bollard has `ssh` feature that uses system ssh
   - What's unclear: Whether it gives enough control over tunnel lifecycle
   - Recommendation: Start with manual tunnel for full control; evaluate Bollard SSH feature if issues arise

2. **SSH agent locked key handling**
   - What we know: BatchMode=yes prevents interactive passphrase prompts
   - What's unclear: Best UX when key is encrypted and not in agent
   - Recommendation: Detect this scenario and print clear instructions ("Run ssh-add ~/.ssh/id_rsa")

3. **hosts.json vs config.json merge**
   - What we know: Separate file avoids schema changes to main config
   - What's unclear: User preference for single vs multiple config files
   - Recommendation: Use separate `hosts.json` for now; can merge later if users request

## Sources

### Primary (HIGH confidence)
- [Bollard Docker API docs](https://docs.rs/bollard/latest/bollard/struct.Docker.html) - Connection methods, SSH support
- [russh GitHub](https://github.com/Eugeny/russh) - Feature comparison, port forwarding capabilities
- [ssh_jumper crate](https://docs.rs/ssh_jumper/) - Jump host tunnel pattern
- [Docker SSH context docs](https://code.visualstudio.com/docs/containers/ssh) - Docker native SSH support

### Secondary (MEDIUM confidence)
- [SSH port forwarding in Rust](https://dev.to/bbkr/ssh-port-forwarding-from-within-rust-code-5an) - russh tunnel example
- [tokio-retry docs](https://docs.rs/tokio-retry/latest/tokio_retry/strategy/struct.ExponentialBackoff.html) - Backoff strategies
- [rpassword crate](https://github.com/conradkleinespel/rpassword) - Secure password prompts

### Tertiary (LOW confidence)
- Community patterns for Docker over SSH need validation in practice

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - system ssh is well-documented, bollard HTTP connection is straightforward
- Architecture: HIGH - patterns derived from codebase conventions + CONTEXT.md constraints
- Pitfalls: MEDIUM - some based on general SSH experience, not all tested in this specific context

**Research date:** 2026-01-23
**Valid until:** 2026-02-23 (stable patterns, unlikely to change)
