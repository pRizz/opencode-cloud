# Phase 10: Remote Administration via Cockpit - Research

**Researched:** 2026-01-22
**Domain:** Cockpit web console integration, systemd in Docker, PAM authentication
**Confidence:** HIGH

## Summary

This phase integrates Cockpit into the Docker container to provide web-based administration alongside the CLI. The key architectural insight is that Cockpit authenticates via PAM - the same users created with `occ user add` will work for Cockpit login. This requires running systemd inside the container to support Cockpit's socket activation and service management.

The existing Dockerfile uses `tini` as init and runs a single opencode process. For Cockpit integration, the container must switch to systemd as PID 1, which requires specific Docker run flags (`--privileged` or limited capabilities, cgroup mounts). Cockpit packages are available in Ubuntu 24.04 repositories, with backports providing the latest version (339+).

**Primary recommendation:** Install `cockpit-ws` and `cockpit-system` packages (minimal set includes terminal). Use systemd as container init with proper cgroup mounts. Configure Cockpit port via systemd socket drop-in. Cockpit logs will appear in journald, viewable via `occ logs` alongside other container logs.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Package | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| cockpit-ws | 339+ (backports) | Web service and login page | Required for web access |
| cockpit-system | 339+ (backports) | System overview, services, terminal | Provides core admin features |
| cockpit-bridge | 339+ (backports) | Backend communication | Required dependency |
| systemd | 255+ (Ubuntu 24.04) | Init system and service manager | Required for Cockpit operation |
| dbus | 1.14+ (Ubuntu 24.04) | Inter-process communication | Required for systemd/Cockpit |

### Supporting
| Package | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cockpit-networkmanager | 339+ | Network configuration UI | Full mode only |
| cockpit-storaged | 339+ | Storage management UI | Full mode only |
| cockpit-packagekit | 339+ | Package updates UI | Full mode only |
| cockpit-podman | varies | Container management | If DinD enabled |

### Minimal vs Full Mode

**Minimal (default):** `cockpit-ws`, `cockpit-system`, `cockpit-bridge`
- System overview
- Terminal access
- Service management (start/stop/restart)
- Logs viewer
- ~50MB additional image size

**Full:** All cockpit-* packages
- Everything in minimal plus:
- Network configuration (requires NetworkManager)
- Storage management (requires udisks2)
- Package updates
- ~150MB+ additional image size

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| systemd in Docker | supervisord + custom init | Cockpit requires systemd; no alternative |
| cockpit-system | separate terminal app | Cockpit integrates better, same auth |
| PAM auth | separate auth system | PAM is already working, Cockpit uses it natively |

**Installation (in Dockerfile):**
```bash
# From backports for latest version
. /etc/os-release
apt-get install -t ${VERSION_CODENAME}-backports \
    cockpit-ws \
    cockpit-system \
    cockpit-bridge \
    systemd \
    systemd-sysv \
    dbus
```

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── docker/
│   ├── Dockerfile           # MODIFY: Add Cockpit packages, systemd setup
│   ├── container.rs         # MODIFY: Add Cockpit port mapping
│   └── cockpit.rs           # NEW: Cockpit health check, status
├── config/
│   └── schema.rs            # MODIFY: Add cockpit_port, cockpit_enabled, cockpit_mode

packages/cli-rust/src/
├── commands/
│   ├── cockpit.rs           # NEW: occ cockpit command
│   ├── status.rs            # MODIFY: Show Cockpit URL and health
│   └── start.rs             # MODIFY: Output Cockpit availability
└── wizard/
    └── ports.rs             # MODIFY: Prompt for Cockpit port
```

### Pattern 1: systemd as Container Init
**What:** Run systemd as PID 1 inside Docker container
**When to use:** Required for Cockpit to function
**Example:**
```dockerfile
# Source: https://www.codegenes.net/blog/how-can-systemd-and-systemctl-be-enabled-and-used-in-ubuntu-docker-containers/
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Install systemd and required components
RUN apt-get update && apt-get install -y --no-install-recommends \
    systemd \
    systemd-sysv \
    dbus \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services to reduce startup time
RUN systemctl mask \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount \
    systemd-update-utmp.service \
    systemd-tmpfiles-setup.service \
    systemd-remount-fs.service

# Required volumes for systemd
VOLUME ["/sys/fs/cgroup", "/run", "/tmp"]

# systemd as init
CMD ["/sbin/init"]
```

### Pattern 2: Cockpit Port Configuration via systemd Drop-in
**What:** Configure Cockpit to listen on custom port
**When to use:** User changes cockpit_port from default 9090
**Example:**
```bash
# Create drop-in directory and config
mkdir -p /etc/systemd/system/cockpit.socket.d
cat > /etc/systemd/system/cockpit.socket.d/listen.conf << 'EOF'
[Socket]
ListenStream=
ListenStream=9090
EOF

# Reload and restart
systemctl daemon-reload
systemctl restart cockpit.socket
```

**Implementation in Rust:**
```rust
// Source: Cockpit documentation + existing exec pattern
pub async fn configure_cockpit_port(
    client: &DockerClient,
    container: &str,
    port: u16,
) -> Result<(), DockerError> {
    // Create drop-in directory
    let mkdir_cmd = vec![
        "mkdir", "-p",
        "/etc/systemd/system/cockpit.socket.d"
    ];
    exec_command(client, container, mkdir_cmd).await?;

    // Write port configuration
    let config = format!(
        "[Socket]\nListenStream=\nListenStream={}\n",
        port
    );
    let write_cmd = vec![
        "sh", "-c",
        &format!(
            "echo '{}' > /etc/systemd/system/cockpit.socket.d/listen.conf",
            config
        )
    ];
    exec_command(client, container, write_cmd).await?;

    // Reload systemd and restart socket
    exec_command(client, container, vec!["systemctl", "daemon-reload"]).await?;
    exec_command(client, container, vec!["systemctl", "restart", "cockpit.socket"]).await?;

    Ok(())
}
```

### Pattern 3: Cockpit Health Check
**What:** Check if Cockpit is running and accessible
**When to use:** `occ status` command
**Example:**
```rust
// Source: Existing health check pattern in status.rs
pub async fn check_cockpit_health(
    client: &DockerClient,
    container: &str,
    port: u16,
) -> Result<CockpitStatus, DockerError> {
    // Check if cockpit.socket is active
    let output = exec_command(
        client,
        container,
        vec!["systemctl", "is-active", "cockpit.socket"]
    ).await;

    let socket_active = output.map(|s| s.trim() == "active").unwrap_or(false);

    if !socket_active {
        return Ok(CockpitStatus::Disabled);
    }

    // Check if accessible (socket activates service on demand)
    let curl_output = exec_command(
        client,
        container,
        vec!["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}",
             &format!("http://127.0.0.1:{}/", port)]
    ).await;

    match curl_output {
        Ok(code) if code.trim() == "200" || code.trim() == "302" => {
            Ok(CockpitStatus::Running)
        }
        _ => Ok(CockpitStatus::NotResponding)
    }
}

pub enum CockpitStatus {
    Running,
    Disabled,
    NotResponding,
}
```

### Pattern 4: Container Run Flags for systemd
**What:** Docker run flags required for systemd to work
**When to use:** Container creation
**Example:**
```rust
// Source: Docker systemd best practices
use bollard::service::{HostConfig, Mount, MountTypeEnum};
use bollard::container::Config;

pub fn create_systemd_host_config(cockpit_port: u16) -> HostConfig {
    HostConfig {
        // Required for systemd
        privileged: Some(false),  // Avoid if possible
        cap_add: Some(vec![
            "SYS_ADMIN".to_string(),  // Required for systemd cgroup access
        ]),
        // tmpfs for /run and /tmp
        tmpfs: Some(std::collections::HashMap::from([
            ("/run".to_string(), "".to_string()),
            ("/tmp".to_string(), "".to_string()),
        ])),
        // cgroup mount (read-only for security)
        mounts: Some(vec![
            Mount {
                target: Some("/sys/fs/cgroup".to_string()),
                source: Some("/sys/fs/cgroup".to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            },
        ]),
        // Port mappings
        port_bindings: Some(/* add cockpit_port */),
        ..Default::default()
    }
}
```

### Pattern 5: opencode as systemd Service
**What:** Run opencode as a systemd service alongside Cockpit
**When to use:** Container startup
**Example:**
```bash
# /etc/systemd/system/opencode.service
[Unit]
Description=opencode Web Interface
After=network.target

[Service]
Type=simple
User=opencode
WorkingDirectory=/home/opencode/workspace
ExecStart=/home/opencode/.opencode/bin/opencode web --port 3000 --hostname 0.0.0.0
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Anti-Patterns to Avoid
- **Using --privileged in production:** Grants full access to host devices; use limited capabilities instead
- **Running Cockpit without systemd:** Cockpit requires systemd socket activation; it will not work otherwise
- **Exposing Cockpit without authentication:** Always ensure users are configured before network exposure
- **Changing port via cockpit.conf:** Port must be configured via systemd socket drop-in, not cockpit.conf

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Web terminal | Custom websocket terminal | Cockpit terminal | Already integrated, PAM auth works |
| Service management UI | Custom REST API | Cockpit services | systemd integration, no code needed |
| Log viewing | Custom log parser | Cockpit logs/journalctl | journald integration built-in |
| Port configuration | Direct socket manipulation | systemd drop-in | Cockpit expects systemd config |
| Init system | tini/dumb-init | systemd | Cockpit requires systemd |

**Key insight:** Cockpit provides everything needed for container administration. The only custom code required is configuration, health checks, and CLI commands to expose Cockpit to users.

## Common Pitfalls

### Pitfall 1: Container Exits Immediately with systemd
**What goes wrong:** Container starts and immediately exits
**Why it happens:** Missing cgroup mount or tmpfs for /run
**How to avoid:** Always include:
- `--tmpfs /run --tmpfs /tmp`
- `-v /sys/fs/cgroup:/sys/fs/cgroup:ro`
**Warning signs:** Container status shows "Exited (1)" immediately after start

### Pitfall 2: Cockpit Login Fails Despite Valid User
**What goes wrong:** User created with `occ user add` cannot log into Cockpit
**Why it happens:** User shell set to /bin/bash but Cockpit expects specific shell
**How to avoid:** Ensure users have valid login shells (`/bin/bash` or `/bin/zsh`)
**Warning signs:** "Not Found" error after login, or login screen reappears

### Pitfall 3: Cockpit Not Accessible on Custom Port
**What goes wrong:** After changing cockpit_port, Cockpit unreachable
**Why it happens:** Port changed in config but not in systemd socket
**How to avoid:** Always use systemd drop-in pattern; `systemctl daemon-reload && systemctl restart cockpit.socket`
**Warning signs:** Connection refused on new port, works on 9090

### Pitfall 4: Services Page Empty
**What goes wrong:** Cockpit services page shows no services
**Why it happens:** cockpit-system not installed or dbus not running
**How to avoid:** Verify `apt install cockpit-system` and `systemctl status dbus`
**Warning signs:** "Error loading services" in Cockpit

### Pitfall 5: Excessive Image Size with Full Mode
**What goes wrong:** Image grows 200MB+ when full mode enabled
**Why it happens:** Full mode pulls NetworkManager, udisks2, PackageKit dependencies
**How to avoid:** Use minimal mode by default; document full mode size impact
**Warning signs:** Slow image pulls, disk space warnings

### Pitfall 6: Cockpit Times Out During Long Operations
**What goes wrong:** Cockpit web service exits, browser shows disconnect
**Why it happens:** cockpit-ws exits after 90 seconds without login, 30 seconds without activity
**How to avoid:** This is expected behavior; document that Cockpit restarts on demand via socket activation
**Warning signs:** "Connection lost" after idle period (normal)

## Code Examples

Verified patterns from official sources:

### Dockerfile Additions for Cockpit
```dockerfile
# Source: https://cockpit-project.org/running + systemd Docker patterns

# Install systemd and Cockpit (minimal mode)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends \
    systemd \
    systemd-sysv \
    dbus \
    && rm -rf /var/lib/apt/lists/*

# Install Cockpit from backports
RUN . /etc/os-release && \
    apt-get update && \
    apt-get install -t ${VERSION_CODENAME}-backports -y --no-install-recommends \
    cockpit-ws \
    cockpit-system \
    cockpit-bridge \
    && rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services
RUN systemctl mask \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount \
    systemd-update-utmp.service \
    systemd-tmpfiles-setup.service

# Enable Cockpit socket
RUN systemctl enable cockpit.socket

# Create opencode systemd service
COPY opencode.service /etc/systemd/system/
RUN systemctl enable opencode.service

# Required volumes
VOLUME ["/sys/fs/cgroup", "/run", "/tmp"]

# Expose Cockpit port (default)
EXPOSE 9090

# Change entrypoint to systemd
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
```

### cockpit.conf for HTTP and Proxy Headers
```ini
# /etc/cockpit/cockpit.conf
# Source: https://cockpit-project.org/guide/latest/cockpit.conf.5

[WebService]
# Allow HTTP connections (TLS terminated externally)
AllowUnencrypted = true

# Trust proxy headers for X-Forwarded-For, X-Forwarded-Proto
ProtocolHeader = X-Forwarded-Proto
ForwardedForHeader = X-Forwarded-For

# Limit concurrent login attempts
MaxStartups = 10
```

### Container Creation with Cockpit
```rust
// Source: bollard docs + existing container.rs patterns
pub async fn create_container_with_cockpit(
    docker: &Docker,
    config: &Config,
) -> Result<String, DockerError> {
    let mut port_bindings = HashMap::new();

    // opencode web port
    port_bindings.insert(
        "3000/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some(config.bind_address.clone()),
            host_port: Some(config.port.to_string()),
        }]),
    );

    // Cockpit port (if enabled)
    if config.cockpit_enabled {
        port_bindings.insert(
            format!("{}/tcp", config.cockpit_port),
            Some(vec![PortBinding {
                host_ip: Some(config.bind_address.clone()),
                host_port: Some(config.cockpit_port.to_string()),
            }]),
        );
    }

    let host_config = HostConfig {
        cap_add: Some(vec!["SYS_ADMIN".to_string()]),
        tmpfs: Some(HashMap::from([
            ("/run".to_string(), "".to_string()),
            ("/tmp".to_string(), "".to_string()),
        ])),
        binds: Some(vec![
            "/sys/fs/cgroup:/sys/fs/cgroup:ro".to_string(),
        ]),
        port_bindings: Some(port_bindings),
        // ... other existing config
        ..Default::default()
    };

    // ... create container
}
```

### occ cockpit Command
```rust
// Source: Existing command patterns
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Open Cockpit web console in browser")]
pub struct CockpitCommand {}

impl CockpitCommand {
    pub async fn run(&self, config: &Config) -> Result<()> {
        // Check if container is running
        let docker = DockerClient::new()?;
        let container_running = check_container_status(&docker).await?;

        if !container_running {
            bail!(
                "Container is not running.\n\
                 Start it with: occ start\n\
                 Then access Cockpit at: http://{}:{}",
                config.bind_address,
                config.cockpit_port
            );
        }

        if !config.cockpit_enabled {
            bail!(
                "Cockpit is disabled.\n\
                 Enable it with: occ config set cockpit_enabled true\n\
                 Then rebuild: occ rebuild"
            );
        }

        // Open in browser
        let url = format!("http://{}:{}", config.bind_address, config.cockpit_port);
        println!("Opening Cockpit at: {}", url);
        open::that(&url)?;

        Ok(())
    }
}
```

### Config Schema Additions
```rust
// Source: Existing schema.rs pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // ... existing fields ...

    /// Cockpit web console port (default: 9090)
    #[serde(default = "default_cockpit_port")]
    pub cockpit_port: u16,

    /// Enable Cockpit web console (default: true)
    #[serde(default = "default_cockpit_enabled")]
    pub cockpit_enabled: bool,

    /// Cockpit installation mode: "minimal" or "full" (default: minimal)
    /// Requires rebuild to change
    #[serde(default = "default_cockpit_mode")]
    pub cockpit_mode: CockpitMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CockpitMode {
    #[default]
    Minimal,
    Full,
}

fn default_cockpit_port() -> u16 { 9090 }
fn default_cockpit_enabled() -> bool { true }
fn default_cockpit_mode() -> CockpitMode { CockpitMode::Minimal }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Cockpit 314 (Ubuntu default) | Cockpit 339+ (backports) | 2025 | Use backports for latest features |
| Docker with --privileged | Limited CAP_SYS_ADMIN | Best practice | More secure container |
| tini as init | systemd as init | Required for Cockpit | Enables full systemd ecosystem |
| Manual port config | systemd socket drop-in | Cockpit standard | Proper integration |

**Deprecated/outdated:**
- Port configuration in cockpit.conf: Does not work; must use systemd socket drop-in
- Running Cockpit without systemd: Not supported
- cockpit-dashboard for multi-host: Being phased out in favor of host switcher

## Open Questions

Things that couldn't be fully resolved:

1. **Exact CAP_SYS_ADMIN Necessity**
   - What we know: systemd needs cgroup access
   - What's unclear: Whether minimal capabilities suffice without SYS_ADMIN
   - Recommendation: Start with CAP_SYS_ADMIN; test with more restrictive caps later

2. **Cockpit + opencode Process Coordination**
   - What we know: Both need to run as systemd services
   - What's unclear: Startup ordering, dependency configuration
   - Recommendation: opencode.service should have `After=network.target`, no explicit Cockpit dependency

3. **Log Integration for `occ logs`**
   - What we know: Cockpit logs to journald
   - What's unclear: Best UX for mixed opencode/Cockpit logs
   - Recommendation: Default to opencode logs only; `occ logs --all` or `occ logs --cockpit` for Cockpit

4. **Security Warning Consistency**
   - What we know: opencode shows network exposure warning
   - What's unclear: Should Cockpit access trigger same warning?
   - Recommendation: Use same warning logic; Cockpit follows same bind_address

## Sources

### Primary (HIGH confidence)
- [Cockpit Running Guide](https://cockpit-project.org/running) - Installation, backports
- [Cockpit cockpit.conf](https://cockpit-project.org/guide/latest/cockpit.conf.5) - All configuration options
- [Cockpit TCP Port and Address](https://cockpit-project.org/guide/latest/listen) - systemd socket configuration
- [Cockpit Authentication](https://cockpit-project.org/guide/latest/authentication) - PAM integration
- [Cockpit Startup](https://cockpit-project.org/guide/latest/startup) - systemd socket activation
- [Ubuntu 24.04 Cockpit Package](https://launchpad.net/ubuntu/noble/+source/cockpit) - Package availability
- [systemd in Docker Guide](https://www.codegenes.net/blog/how-can-systemd-and-systemctl-be-enabled-and-used-in-ubuntu-docker-containers/) - Docker + systemd best practices

### Secondary (MEDIUM confidence)
- [Vultr Cockpit Ubuntu 24.04 Guide](https://docs.vultr.com/how-to-install-cockpit-on-ubuntu-24-04) - Installation steps
- [Cockpit Minimal Install Discussion](https://github.com/cockpit-project/cockpit/discussions/16490) - Package recommendations
- [Is Cockpit Secure](https://cockpit-project.org/blog/is-cockpit-secure.html) - Security architecture
- [Red Hat systemd in Container](https://developers.redhat.com/blog/2019/04/24/how-to-run-systemd-in-a-container) - systemd patterns

### Tertiary (LOW confidence)
- WebSearch results for rate limiting - Cockpit relies on PAM; no built-in rate limiting
- Community discussions on shell issues - Document edge cases

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Cockpit packages well-documented for Ubuntu 24.04
- Architecture: HIGH - systemd in Docker well-established pattern
- Pitfalls: HIGH - Common issues well-documented in Cockpit issues/discussions
- PAM integration: HIGH - Cockpit uses PAM natively; same users will work

**Research date:** 2026-01-22
**Valid until:** 2026-02-22 (30 days - Cockpit stable, systemd patterns stable)
