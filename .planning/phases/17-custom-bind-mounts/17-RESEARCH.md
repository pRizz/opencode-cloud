# Phase 17: Custom Bind Mounts - Research

**Researched:** 2026-01-25
**Domain:** Docker bind mounts, CLI configuration management, path validation
**Confidence:** HIGH

## Summary

This phase adds custom bind mount support to allow users to mount local filesystem directories into the Docker container. The existing codebase already uses Bollard 0.18 for Docker operations with `Mount` structs for named volumes. Bind mounts use the same `Mount` struct with `MountTypeEnum::BIND` instead of `MountTypeEnum::VOLUME`.

The implementation extends the existing config schema to store bind mounts as an array of strings in the standard Docker format (`/host/path:/container/path[:ro]`). A new `mount` subcommand group (`occ mount add/remove/list`) follows the established pattern from `occ user` and `occ config env`. The start command gains `--mount` and `--no-mounts` flags for one-time mount overrides.

**Primary recommendation:** Implement bind mounts by extending the existing `Mount` vector in `create_container()`, reusing Bollard's `MountTypeEnum::BIND` type. Use `std::fs::canonicalize()` for path validation and symlink resolution.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bollard | 0.18 | Docker API client | Already used; provides `Mount`, `MountTypeEnum::BIND` |
| std::fs | N/A | Path validation | Rust stdlib; `canonicalize()`, `metadata()` |
| std::path | N/A | Path manipulation | Rust stdlib; `Path`, `PathBuf`, `is_absolute()` |
| serde | 1.0 | Config serialization | Already used for config schema |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| comfy-table | existing | Table output | For `occ mount list` output |
| clap | 4.5 | CLI argument parsing | For new `mount` subcommand |
| console | 0.16 | Styled terminal output | For error messages and status display |
| thiserror | 2 | Error types | For mount-specific errors |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| String parsing | Struct-based mount config | Strings simpler for Docker-familiar users; matches docker-compose format |
| Config array | Separate mounts.json file | Array in config simpler; no new file needed |

**Installation:**
No new dependencies required - all libraries already in `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── config/
│   └── schema.rs         # Add `mounts: Vec<String>` field
├── docker/
│   └── container.rs      # Extend create_container() for bind mounts
│   └── mount.rs          # NEW: Mount parsing and validation

packages/cli-rust/src/
├── commands/
│   └── mount/            # NEW: mount subcommand group
│       ├── mod.rs        # MountArgs, MountCommands enum
│       ├── add.rs        # occ mount add
│       ├── remove.rs     # occ mount remove
│       └── list.rs       # occ mount list
│   └── start.rs          # Add --mount and --no-mounts flags
│   └── status.rs         # Add Mounts section
│   └── mod.rs            # Register mount subcommand
├── lib.rs                # Register Mount in Commands enum
```

### Pattern 1: Mount String Parsing
**What:** Parse Docker-style mount strings into structured data
**When to use:** Processing user input and config values
**Example:**
```rust
// Source: CONTEXT.md decision format
pub struct ParsedMount {
    pub host_path: PathBuf,
    pub container_path: String,
    pub read_only: bool,
}

impl ParsedMount {
    /// Parse mount string in format: /host/path:/container/path[:ro]
    pub fn parse(mount_str: &str) -> Result<Self, MountError> {
        let parts: Vec<&str> = mount_str.split(':').collect();

        // Handle 2 parts (rw) or 3 parts (with :ro/:rw)
        let (host_path, container_path, read_only) = match parts.len() {
            2 => (parts[0], parts[1], false),
            3 => {
                let ro = match parts[2] {
                    "ro" => true,
                    "rw" => false,
                    _ => return Err(MountError::InvalidFormat(mount_str.to_string())),
                };
                (parts[0], parts[1], ro)
            }
            _ => return Err(MountError::InvalidFormat(mount_str.to_string())),
        };

        let host = PathBuf::from(host_path);
        if !host.is_absolute() {
            return Err(MountError::RelativePath(host_path.to_string()));
        }

        Ok(Self {
            host_path: host,
            container_path: container_path.to_string(),
            read_only,
        })
    }
}
```

### Pattern 2: Bollard Mount Construction
**What:** Convert parsed mount to Bollard Mount struct for container creation
**When to use:** In create_container when adding mounts to HostConfig
**Example:**
```rust
// Source: Bollard docs + existing container.rs pattern
use bollard::service::{Mount, MountTypeEnum};

impl ParsedMount {
    pub fn to_bollard_mount(&self) -> Mount {
        Mount {
            target: Some(self.container_path.clone()),
            source: Some(self.host_path.to_string_lossy().to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(self.read_only),
            ..Default::default()
        }
    }
}
```

### Pattern 3: Subcommand Structure (from user/mod.rs)
**What:** Subcommand group with Args and Commands enum
**When to use:** For `occ mount` subcommand group
**Example:**
```rust
// Source: packages/cli-rust/src/commands/user/mod.rs
#[derive(Args)]
pub struct MountArgs {
    #[command(subcommand)]
    pub command: MountCommands,
}

#[derive(Subcommand)]
pub enum MountCommands {
    /// Add a new bind mount
    Add(MountAddArgs),
    /// Remove a bind mount
    Remove(MountRemoveArgs),
    /// List configured bind mounts
    List(MountListArgs),
}
```

### Anti-Patterns to Avoid
- **Validating only at config time:** Paths can become stale between config and start; validate at both times
- **Auto-creating missing directories:** Docker `--mount` errors on missing paths (unlike `-v`); match this behavior
- **Silently mounting over system paths:** Warn but don't block `/etc`, `/usr`, etc. per CONTEXT.md

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Path canonicalization | Manual symlink resolution | `std::fs::canonicalize()` | Handles symlinks, `.`, `..`, relative paths |
| Path existence check | Manual stat | `std::fs::metadata()` | Returns error for non-existent paths |
| Mount struct | Custom Docker API calls | `bollard::service::Mount` | Type-safe, already used in codebase |
| Table output | Custom formatting | `comfy_table` | Already used for user list |

**Key insight:** Bollard already has all the types needed (`Mount`, `MountTypeEnum::BIND`). The existing `create_container` function already constructs a `Vec<Mount>` for volumes - bind mounts are just additional entries.

## Common Pitfalls

### Pitfall 1: Windows Path Format
**What goes wrong:** Windows paths use backslashes and drive letters (`C:\Users\`), which Docker expects as `/c/Users/`
**Why it happens:** Docker Desktop for Windows uses WSL2 with Linux path conventions
**How to avoid:** On Windows, convert paths: `C:\foo` -> `/mnt/c/foo` or `/c/foo`
**Warning signs:** "invalid mount config" errors on Windows

### Pitfall 2: Docker Desktop File Sharing
**What goes wrong:** Mount fails even with valid path because directory isn't in Docker's allowed file sharing list
**Why it happens:** Docker Desktop restricts what host paths can be mounted for security
**How to avoid:** Document requirement to configure Docker Desktop file sharing; provide clear error message
**Warning signs:** Permission denied or "not shared" errors

### Pitfall 3: Path Existence vs. Permissions
**What goes wrong:** Path exists but isn't readable/writable by container user
**Why it happens:** Container runs as different UID than host user
**How to avoid:** Check `metadata()` success at validation time; document permissions requirements
**Warning signs:** Empty directories after mount, permission denied inside container

### Pitfall 4: Relative Paths
**What goes wrong:** User specifies `./project:/workspace/project` expecting CWD resolution
**Why it happens:** Relative paths are ambiguous - relative to what?
**How to avoid:** Reject relative paths with clear error: "Mount paths must be absolute. Use: /full/path/to/dir"
**Warning signs:** Mount string parsing fails or mounts wrong directory

### Pitfall 5: Container Already Running
**What goes wrong:** User adds mount but container already exists with different mount config
**Why it happens:** Docker mounts are set at container creation, not modifiable at runtime
**How to avoid:** Warn user that restart is required; `occ status` shows configured vs. active mounts
**Warning signs:** New mount doesn't appear in container

### Pitfall 6: Symlink Source Path
**What goes wrong:** User mounts a symlink, expects symlink to be mounted
**Why it happens:** `canonicalize()` follows symlinks to real path
**How to avoid:** Per CONTEXT.md, follow symlinks (mount target directory); document behavior
**Warning signs:** Different path shown in status than what user configured

## Code Examples

Verified patterns from the existing codebase:

### Config Schema Extension
```rust
// Source: packages/core/src/config/schema.rs pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // ... existing fields ...

    /// Bind mounts to apply when starting the container
    /// Format: ["/host/path:/container/path", "/host:/mnt:ro"]
    #[serde(default)]
    pub mounts: Vec<String>,
}
```

### Container Creation with Bind Mounts
```rust
// Source: packages/core/src/docker/container.rs existing pattern
use bollard::service::{Mount, MountTypeEnum};

// In create_container(), after existing volume mounts:
let mut mounts = vec![
    // Existing volume mounts...
    Mount {
        target: Some(MOUNT_SESSION.to_string()),
        source: Some(VOLUME_SESSION.to_string()),
        typ: Some(MountTypeEnum::VOLUME),
        read_only: Some(false),
        ..Default::default()
    },
    // ... other volumes
];

// Add bind mounts from config/CLI
for parsed_mount in bind_mounts {
    mounts.push(Mount {
        target: Some(parsed_mount.container_path.clone()),
        source: Some(parsed_mount.host_path.to_string_lossy().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(parsed_mount.read_only),
        ..Default::default()
    });
}
```

### Path Validation
```rust
// Source: std::fs docs + CONTEXT.md decisions
use std::fs;
use std::path::Path;

const SYSTEM_PATHS: &[&str] = &["/etc", "/usr", "/bin", "/sbin", "/lib", "/var"];

pub fn validate_mount_path(host_path: &Path) -> Result<PathBuf, MountError> {
    // Must be absolute
    if !host_path.is_absolute() {
        return Err(MountError::RelativePath(host_path.display().to_string()));
    }

    // Must exist and be a directory
    let canonical = fs::canonicalize(host_path)
        .map_err(|e| MountError::PathNotFound(host_path.display().to_string(), e.to_string()))?;

    let metadata = fs::metadata(&canonical)
        .map_err(|e| MountError::PathNotFound(canonical.display().to_string(), e.to_string()))?;

    if !metadata.is_dir() {
        return Err(MountError::NotADirectory(canonical.display().to_string()));
    }

    Ok(canonical)
}

pub fn check_container_path_warning(container_path: &str) -> Option<String> {
    for system_path in SYSTEM_PATHS {
        if container_path.starts_with(system_path) {
            return Some(format!(
                "Warning: Mounting over system path '{container_path}' may break container functionality"
            ));
        }
    }
    None
}
```

### CLI Flag Pattern
```rust
// Source: packages/cli-rust/src/commands/start.rs pattern
#[derive(Args)]
pub struct StartArgs {
    // ... existing args ...

    /// Add one-time bind mount (can be specified multiple times)
    /// Format: /host/path:/container/path[:ro]
    #[arg(long = "mount", action = clap::ArgAction::Append)]
    pub mounts: Vec<String>,

    /// Skip configured mounts (only use --mount if specified)
    #[arg(long)]
    pub no_mounts: bool,
}
```

### Status Display Pattern
```rust
// Source: packages/cli-rust/src/commands/status.rs pattern
fn display_mounts_section(mounts: &[(String, String, bool, &str)]) {
    println!();
    println!("{}", style("Mounts").bold());
    println!("{}", style("------").dim());

    for (host, container, read_only, source) in mounts {
        let mode = if *read_only { "ro" } else { "rw" };
        let source_tag = match *source {
            "config" => style("(config)").dim(),
            "cli" => style("(cli)").cyan(),
            _ => style("").dim(),
        };
        println!(
            "  {} -> {} {} {}",
            style(host).cyan(),
            container,
            style(mode).dim(),
            source_tag
        );
    }
}
```

### Error Display Pattern
```rust
// Source: packages/cli-rust/src/output/errors.rs pattern + CONTEXT.md
pub fn format_mount_error(path: &str) -> String {
    format!(
        "{}\n  {}\n\n{}\n",
        style("Error: Mount path not found").red().bold(),
        path,
        style(format!("Did the directory move? Run: occ mount remove {path}")).dim()
    )
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `-v` flag for mounts | `--mount` flag | Docker 17.06+ | `--mount` errors on missing paths vs `-v` auto-creates |
| Absolute paths only | Relative paths supported | Docker 23+ | Still recommend absolute for clarity |

**Deprecated/outdated:**
- Docker `-v` flag behavior: Auto-creates missing directories, which can cause silent failures. The `--mount` approach (which Bollard uses) is more explicit.

## Open Questions

Things that couldn't be fully resolved:

1. **Docker Desktop file sharing on macOS**
   - What we know: Docker Desktop has file sharing restrictions
   - What's unclear: Exact error message when path isn't shared
   - Recommendation: Test on macOS Docker Desktop; document requirements

2. **Windows path translation in WSL2**
   - What we know: Paths need translation (`C:\` -> `/mnt/c/` or `/c/`)
   - What's unclear: Best detection method for Windows vs WSL environment
   - Recommendation: Defer Windows support; document limitation

## Sources

### Primary (HIGH confidence)
- Bollard 0.18 docs - Mount struct, MountTypeEnum::BIND
- Existing codebase - container.rs, user/mod.rs, config/schema.rs patterns
- CONTEXT.md - User decisions on path format, validation, CLI structure

### Secondary (MEDIUM confidence)
- [Docker Docs - Bind Mounts](https://docs.docker.com/engine/storage/bind-mounts/) - Official documentation on bind mount behavior
- [Docker Docs - Workshop Bind Mounts](https://docs.docker.com/get-started/workshop/06_bind_mounts/) - Tutorial on bind mount usage

### Tertiary (LOW confidence)
- Web search results on cross-platform path handling - needs validation on actual platforms

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Using existing libraries (Bollard, std::fs)
- Architecture: HIGH - Following established codebase patterns
- Pitfalls: HIGH - Well-documented Docker behavior
- Cross-platform: LOW - Windows/WSL path handling needs testing

**Research date:** 2026-01-25
**Valid until:** 60 days (stable domain, Bollard version locked)
