# Phase 4: Platform Service Installation - Research

**Researched:** 2026-01-19
**Domain:** systemd (Linux) and launchd (macOS) service registration
**Confidence:** HIGH

## Summary

This phase implements `occ install` and `occ uninstall` commands to register the opencode-cloud service with the host OS's service manager. On Linux, this means generating and managing systemd user service units. On macOS, this means generating and managing launchd user agents via property list (plist) files.

The standard approach is to:
1. Generate platform-specific service definition files (systemd `.service` or launchd `.plist`)
2. Place them in user-level directories (`~/.config/systemd/user/` or `~/Library/LaunchAgents/`)
3. Use platform tools to register/enable the service (`systemctl --user` or `launchctl bootstrap`)
4. Configure restart policies within the service definition files

**Primary recommendation:** Use string templating for service file generation (not heavy libraries), the `plist` crate for macOS plist serialization, and execute platform tools via `std::process::Command` for registration.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| plist | 1.8.x | macOS plist serialization | 23M+ downloads, Serde integration, de facto standard |
| dialoguer | 0.11.x | Interactive prompts | Part of console-rs ecosystem, needed for reinstall confirmation |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::process::Command | std | Execute systemctl/launchctl | All platform tool invocations |
| std::fs | std | File operations | Writing service files |
| directories | 5.x | XDG paths (already in project) | Getting user home/config directories |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| plist crate | Manual XML templating | plist handles edge cases (escaping, encoding) correctly |
| dialoguer | console::Term::read_line | dialoguer provides consistent UX with other prompts |
| systemd-service crate | String templating | systemd-service is too new (87 downloads), string templating is sufficient |

**Installation:**
```bash
# In workspace Cargo.toml
plist = "1.8"
dialoguer = "0.11"
```

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── platform/              # NEW: Platform-specific service management
│   ├── mod.rs            # Platform detection, ServiceManager trait
│   ├── systemd.rs        # Linux systemd implementation
│   └── launchd.rs        # macOS launchd implementation
└── ...

packages/cli-rust/src/
├── commands/
│   ├── install.rs        # NEW: occ install command
│   └── uninstall.rs      # NEW: occ uninstall command
└── ...
```

### Pattern 1: Platform Abstraction via Trait
**What:** Define a `ServiceManager` trait that both systemd and launchd implementations satisfy
**When to use:** When the same high-level operations (install, uninstall, is_installed) need platform-specific implementations
**Example:**
```rust
// Source: Standard Rust pattern for platform abstraction
pub trait ServiceManager {
    fn install(&self, config: &ServiceConfig) -> Result<InstallResult>;
    fn uninstall(&self, remove_data: bool) -> Result<()>;
    fn is_installed(&self) -> Result<bool>;
    fn service_file_path(&self) -> PathBuf;
}

// Platform-specific implementations
#[cfg(target_os = "linux")]
pub fn get_service_manager() -> Box<dyn ServiceManager> {
    Box::new(SystemdManager::new())
}

#[cfg(target_os = "macos")]
pub fn get_service_manager() -> Box<dyn ServiceManager> {
    Box::new(LaunchdManager::new())
}
```

### Pattern 2: Configuration-Driven Service Files
**What:** Store restart policies and other configurable options in the app config, then template them into service files
**When to use:** When service behavior should be user-configurable
**Example:**
```rust
// In config/schema.rs - extend Config struct
pub struct Config {
    // ... existing fields ...

    /// Boot mode: "user" (default) or "system"
    #[serde(default = "default_boot_mode")]
    pub boot_mode: String,

    /// Number of restart attempts on crash (default: 3)
    #[serde(default = "default_restart_retries")]
    pub restart_retries: u32,

    /// Delay between restart attempts in seconds (default: 5)
    #[serde(default = "default_restart_delay")]
    pub restart_delay: u32,
}
```

### Pattern 3: Idempotent Operations with Clear Feedback
**What:** Operations that can be run multiple times safely, with informative messages
**When to use:** Always for install/uninstall operations
**Example:**
```rust
// Idempotent uninstall - exit 0 even if not installed
pub async fn cmd_uninstall(args: &UninstallArgs, quiet: bool) -> Result<()> {
    let manager = get_service_manager();

    if !manager.is_installed()? {
        if !quiet {
            println!("{}", style("Service not installed.").dim());
        }
        return Ok(());  // Exit 0, not an error
    }

    // ... proceed with uninstall ...
}
```

### Anti-Patterns to Avoid
- **Hardcoding paths:** Use `directories` crate and proper path resolution
- **Assuming root access:** Default to user-level service installation
- **Ignoring platform detection:** Always check `cfg!(target_os = "...")` or runtime detection
- **Blocking on I/O in async context:** Use `tokio::fs` or spawn_blocking for file operations

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| plist XML generation | String concatenation | `plist` crate | XML escaping, proper encoding, edge cases |
| Interactive prompts | `stdin().read_line()` | `dialoguer::Confirm` | Consistent styling, input validation, history |
| Home directory detection | `$HOME` env var | `directories::BaseDirs` | Cross-platform, handles edge cases |
| Detecting init system | Parse `/sbin/init` | Check `/run/systemd/system` exists | More reliable on modern systems |

**Key insight:** Service file generation is simple enough for string templating, but plist files have XML encoding requirements that make a proper serialization library worthwhile.

## Common Pitfalls

### Pitfall 1: Forgetting daemon-reload After File Changes
**What goes wrong:** Service file is written but systemd doesn't see the changes
**Why it happens:** systemd caches unit files and needs explicit reload
**How to avoid:** Always run `systemctl --user daemon-reload` after writing/modifying service files
**Warning signs:** "Unit not found" errors even though file exists

### Pitfall 2: launchctl Load vs Bootstrap Confusion
**What goes wrong:** Using deprecated `load`/`unload` commands that behave inconsistently
**Why it happens:** Many online tutorials use legacy commands
**How to avoid:** Use modern `launchctl bootstrap gui/$UID` and `launchctl bootout gui/$UID` syntax
**Warning signs:** Unclear error messages, different behavior as root vs user

### Pitfall 3: User Services Not Starting After Reboot
**What goes wrong:** Service works when manually started but doesn't start on boot
**Why it happens:**
- Linux: User services require lingering enabled OR an active login session
- macOS: Agent must be in `~/Library/LaunchAgents/` with `RunAtLoad` set
**How to avoid:**
- Linux: Enable lingering with `loginctl enable-linger $USER` for true boot persistence
- macOS: Ensure `RunAtLoad` is true in plist
**Warning signs:** Works after login but not after reboot without logging in

### Pitfall 4: System-Level Installation Without Proper Permissions
**What goes wrong:** User tries to install system-level service without sudo
**Why it happens:** User set `boot_mode=system` but didn't escalate privileges
**How to avoid:** Check permissions before attempting, provide clear error with fix instructions
**Warning signs:** "Permission denied" errors writing to `/etc/systemd/system/` or `/Library/LaunchDaemons/`

### Pitfall 5: Service File Escaping Issues
**What goes wrong:** Paths with spaces or special characters break service execution
**Why it happens:** Improper quoting in ExecStart or ProgramArguments
**How to avoid:**
- systemd: Quote paths with spaces, use proper escaping
- launchd: plist crate handles escaping automatically
**Warning signs:** "Executable not found" when path contains spaces

### Pitfall 6: Restart Policy Rate Limiting
**What goes wrong:** Service stops restarting after a few crashes
**Why it happens:** Default rate limits (systemd: 5 starts in 10 seconds, launchd: 10 second throttle)
**How to avoid:** Configure appropriate `StartLimitBurst`/`StartLimitIntervalSec` for systemd, `ThrottleInterval` for launchd
**Warning signs:** Service enters "failed" state and won't restart

## Code Examples

Verified patterns from official sources:

### systemd User Service Unit File Template
```ini
# Source: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html
[Unit]
Description=opencode-cloud container service
Documentation=https://github.com/pRizz/opencode-cloud
After=docker.service
Requires=docker.service

[Service]
Type=simple
ExecStart={executable_path} start --no-daemon
ExecStop={executable_path} stop
Restart=on-failure
RestartSec={restart_delay}s

# Rate limiting: {restart_retries} attempts per {restart_delay * restart_retries} seconds
StartLimitBurst={restart_retries}
StartLimitIntervalSec={restart_delay * restart_retries}

[Install]
WantedBy=default.target
```

### launchd User Agent Plist Structure
```rust
// Source: https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LaunchdAgent {
    pub label: String,
    pub program_arguments: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub run_at_load: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<KeepAliveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttle_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_out_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_error_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeepAliveConfig {
    /// Restart on non-zero exit (crash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful_exit: Option<bool>,
    /// Restart on signal-based crash (SIGSEGV, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crashed: Option<bool>,
}
```

### Writing plist to File
```rust
// Source: https://docs.rs/plist/latest/plist/
use std::fs::File;

fn write_launchd_plist(agent: &LaunchdAgent, path: &Path) -> Result<()> {
    let file = File::create(path)?;
    plist::to_writer_xml(file, agent)?;
    Ok(())
}
```

### Detecting systemd Availability
```rust
// Source: https://linuxhandbook.com/check-if-systemd/
use std::path::Path;

pub fn systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}
```

### Running systemctl Commands
```rust
// Source: https://www.freedesktop.org/software/systemd/man/latest/systemctl.html
use std::process::Command;

pub fn systemctl_user(args: &[&str]) -> Result<Output> {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| anyhow!("Failed to run systemctl: {}", e))
}

// Enable and start in one operation
pub fn install_systemd_service(service_name: &str) -> Result<()> {
    // Reload daemon to pick up new/changed unit files
    systemctl_user(&["daemon-reload"])?;

    // Enable for auto-start on login
    systemctl_user(&["enable", service_name])?;

    // Start immediately
    systemctl_user(&["start", service_name])?;

    Ok(())
}
```

### Running launchctl Commands (Modern Syntax)
```rust
// Source: https://www.alansiu.net/2023/11/15/launchctl-new-subcommand-basics-for-macos/
use std::process::Command;

pub fn get_user_id() -> Result<u32> {
    let output = Command::new("id")
        .arg("-u")
        .output()?;
    let uid_str = String::from_utf8_lossy(&output.stdout);
    uid_str.trim().parse().map_err(|e| anyhow!("Failed to parse UID: {}", e))
}

pub fn launchctl_bootstrap(plist_path: &Path) -> Result<()> {
    let uid = get_user_id()?;
    let domain = format!("gui/{}", uid);

    Command::new("launchctl")
        .args(["bootstrap", &domain, &plist_path.display().to_string()])
        .status()
        .map_err(|e| anyhow!("Failed to bootstrap agent: {}", e))?;

    Ok(())
}

pub fn launchctl_bootout(label: &str) -> Result<()> {
    let uid = get_user_id()?;
    let service_target = format!("gui/{}/{}", uid, label);

    Command::new("launchctl")
        .args(["bootout", &service_target])
        .status()
        .map_err(|e| anyhow!("Failed to bootout agent: {}", e))?;

    Ok(())
}
```

### Confirmation Prompt for Reinstall
```rust
// Source: https://github.com/console-rs/dialoguer
use dialoguer::Confirm;

pub fn confirm_reinstall() -> Result<bool> {
    Confirm::new()
        .with_prompt("Service already installed. Reinstall?")
        .default(false)
        .interact()
        .map_err(|e| anyhow!("Prompt failed: {}", e))
}
```

## Service File Locations

### User-Level (Default - No Root Required)

| Platform | Service File Location | Enable Command |
|----------|----------------------|----------------|
| Linux | `~/.config/systemd/user/opencode-cloud.service` | `systemctl --user enable opencode-cloud` |
| macOS | `~/Library/LaunchAgents/com.opencode-cloud.plist` | `launchctl bootstrap gui/$UID <path>` |

### System-Level (Requires Root)

| Platform | Service File Location | Enable Command |
|----------|----------------------|----------------|
| Linux | `/etc/systemd/system/opencode-cloud.service` | `sudo systemctl enable opencode-cloud` |
| macOS | `/Library/LaunchDaemons/com.opencode-cloud.plist` | `sudo launchctl bootstrap system <path>` |

## Restart Policy Configuration

### systemd Restart Options

| Option | Behavior | Use Case |
|--------|----------|----------|
| `Restart=on-failure` | Restart on non-zero exit or signal | Recommended for services |
| `Restart=always` | Restart regardless of exit status | Use with caution |
| `RestartSec=5s` | Wait 5 seconds between restarts | Prevents rapid cycling |
| `StartLimitBurst=3` | Max 3 restart attempts | Matches requirement default |
| `StartLimitIntervalSec=60` | Within 60 second window | Stop after too many failures |

### launchd Restart Options

| Key | Value | Behavior |
|-----|-------|----------|
| `KeepAlive.SuccessfulExit` | `false` | Restart only on non-zero exit |
| `KeepAlive.Crashed` | `true` | Restart on signal-based crashes |
| `ThrottleInterval` | `5` | Minimum 5 seconds between restarts |

**Note:** launchd does not have a direct equivalent to `StartLimitBurst`. Excessive restarts will trigger throttling but won't permanently stop the service. To implement "stop after N retries," the service itself would need to track crash count.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `launchctl load/unload` | `launchctl bootstrap/bootout` | macOS 10.10 (2014) | Modern syntax provides better error messages |
| System-level services | User-level services | Ongoing trend | No root required, better security isolation |
| SysVinit scripts | systemd units | systemd adoption ~2015 | Declarative, dependency-aware, consistent |

**Deprecated/outdated:**
- `launchctl load -w`: Use `launchctl bootstrap` instead
- `launchctl unload -w`: Use `launchctl bootout` instead
- `/Library/StartupItems/`: Completely deprecated on modern macOS

## Open Questions

Things that couldn't be fully resolved:

1. **launchd retry limit implementation**
   - What we know: launchd doesn't have a built-in equivalent to systemd's `StartLimitBurst`
   - What's unclear: Best approach to implement "stop after N retries" behavior
   - Recommendation: Track crash count in a state file, have service check on startup. Alternative: Accept that launchd will keep retrying with throttling.

2. **Lingering for boot-time start on Linux**
   - What we know: User services only start after login unless lingering is enabled
   - What's unclear: Whether to auto-enable lingering or just document it
   - Recommendation: For user-level install, document that service starts on login. If user wants true boot-time start, they should use `boot_mode=system` or manually enable lingering.

## Sources

### Primary (HIGH confidence)
- [freedesktop.org systemd.service man page](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html) - Authoritative reference for systemd service configuration
- [Apple Developer - Creating Launch Daemons and Agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html) - Official Apple documentation
- [launchd.plist man page](https://keith.github.io/xcode-man-pages/launchd.plist.5.html) - Comprehensive plist key reference
- [plist crate docs](https://docs.rs/plist/latest/plist/) - Rust plist serialization

### Secondary (MEDIUM confidence)
- [ArchWiki systemd/User](https://wiki.archlinux.org/title/Systemd/User) - Excellent community documentation, verified against official docs
- [launchd.info](https://launchd.info/) - Community tutorial, cross-referenced with Apple docs
- [Alan Siu's launchctl guide](https://www.alansiu.net/2023/11/15/launchctl-new-subcommand-basics-for-macos/) - Modern launchctl syntax reference

### Tertiary (LOW confidence)
- Various Stack Overflow and forum posts for edge case handling - marked for validation during implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - plist crate is well-established; dialoguer is part of console-rs ecosystem already used
- Architecture: HIGH - Platform abstraction pattern is well-established in Rust
- Service file formats: HIGH - Verified against official systemd and Apple documentation
- Pitfalls: MEDIUM - Based on combination of official docs and community experience
- launchd retry limits: LOW - No built-in solution found, may need custom implementation

**Research date:** 2026-01-19
**Valid until:** 60 days (systemd and launchd are stable, slow-changing systems)
