# Phase 15: Prebuilt Image Option - Research

**Researched:** 2026-01-24
**Domain:** CLI user experience, Docker image management, configuration persistence
**Confidence:** HIGH

## Summary

This phase adds user choice between pulling prebuilt Docker images (fast, ~2 min) or building from source (customizable, 30-60 min). The codebase already has all the underlying infrastructure from Phase 14 (version detection, image pulling, registry fallback) and Phase 5 (setup wizard). This phase is primarily about surfacing these existing capabilities to users through CLI flags, config options, and prompts.

The implementation involves: (1) adding `image_source` and `update_check` to the config schema, (2) renaming existing rebuild flags and adding a pull flag to the start command, (3) integrating an image source prompt into the setup wizard, (4) modifying the update command to respect config, and (5) enhancing status output to show image provenance.

**Primary recommendation:** Leverage existing `pull_image()`, `build_image()`, and wizard infrastructure. Focus on clean UX flow and proper state management for image provenance tracking.

## Current Implementation Analysis

### Start Command Flow (`packages/cli-rust/src/commands/start.rs`)

The current start command flow:
1. Connect to Docker
2. Load config for port/bind_address
3. Version compatibility check (if image exists and not rebuilding)
4. Security check (block if no users configured)
5. Handle rebuild if `--cached-rebuild` or `--full-rebuild`
6. Check if image exists; if not, build it
7. Start container
8. Wait for service ready

**Key insight:** The current flow always builds if image doesn't exist. This phase needs to insert "pull or build" decision point before step 6.

**Current flags:**
```rust
pub struct StartArgs {
    pub port: Option<u16>,
    pub open: bool,
    pub no_daemon: bool,
    pub cached_rebuild: bool,      // Rebuild with cache
    pub full_rebuild: bool,        // Rebuild without cache
    pub ignore_version: bool,      // Skip version check
}
```

**Flag renaming required:**
- `--cached_rebuild` -> `--cached-rebuild-sandbox-image`
- `--full_rebuild` -> `--full-rebuild-sandbox-image`
- Add: `--pull-sandbox-image`
- Add: `--no-update-check`

### Version Checking (`packages/core/src/docker/version.rs`)

Current implementation:
```rust
// Constants
pub const VERSION_LABEL: &str = "org.opencode-cloud.version";

// Functions
pub fn get_cli_version() -> &'static str
pub async fn get_image_version(client, image_name) -> Result<Option<String>>
pub fn versions_compatible(cli_version, image_version) -> bool
```

Version compatibility rules:
- `None` (no label) = local build, assume compatible
- `"dev"` = dev build, assume compatible
- Exact match required for versioned images

**Gap:** No registry provenance tracking. Need to add metadata about where image came from.

### Image Operations (`packages/core/src/docker/image.rs`)

Current implementation:
```rust
pub async fn image_exists(client, image, tag) -> Result<bool>
pub async fn build_image(client, tag, progress, no_cache) -> Result<String>
pub async fn pull_image(client, tag, progress) -> Result<String>
```

**Pull retry behavior:**
- Manual retry loop (3 attempts max)
- Exponential backoff: 1s, 2s, 4s
- GHCR first, Docker Hub fallback

**Key insight:** `pull_image()` already handles both registries and returns the full image name with registry prefix. This can be used to track provenance.

### Update Command (`packages/cli-rust/src/commands/update.rs`)

Current flow:
1. Stop service
2. Back up current image as `previous` tag
3. Pull latest image
4. Recreate container
5. Recreate users

**Gap:** Always pulls. Needs to check `image_source` config and rebuild if set to `build`.

### Config Schema (`packages/core/src/config/schema.rs`)

Current fields do not include `image_source` or `update_check`. Schema uses `#[serde(deny_unknown_fields)]` for strict validation.

New fields to add:
```rust
/// Source of Docker image: "prebuilt" | "build"
#[serde(default = "default_image_source")]
pub image_source: String,

/// When to check for updates: "always" | "once" | "never"
#[serde(default = "default_update_check")]
pub update_check: String,
```

### Setup Wizard (`packages/cli-rust/src/wizard/mod.rs`)

Current wizard flow:
1. Prechecks (TTY, Docker)
2. Show current config if exists, ask to reconfigure
3. Quick setup offer
4. Collect auth credentials
5. Collect port/bind (if not quick)
6. Summary and confirm
7. Create user in container if running
8. Return config

**Integration point:** Image source prompt should go after quick setup offer (step 3.5) since it affects both quick and custom flows.

### Status Command (`packages/cli-rust/src/commands/status.rs`)

Currently shows:
- State, URL, Health
- Container name/ID, Image name
- CLI version, Image version (from label)
- Uptime, Port
- Cockpit info (if enabled)
- Security section

**Enhancement needed:** Show image provenance:
```
Image:       v1.0.12 (prebuilt from docker.io)
```
or
```
Image:       v1.0.12 (built from source)
```

## Standard Stack

### Core (already in project)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bollard | * | Docker API client | Used throughout for image/container ops |
| dialoguer | * | Interactive prompts | Used in wizard, Select/Confirm dialogs |
| console | * | Terminal styling | style() for colored output |
| serde | * | Config serialization | JSON config with defaults |

### Supporting (already in project)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio | * | Async runtime | All Docker operations |
| anyhow | * | Error handling | CLI error propagation |
| clap | * | CLI parsing | Arg definitions |

**No new dependencies required** - all needed functionality exists.

## Architecture Patterns

### Recommended Changes to Project Structure

No new files needed. Changes to existing files:

```
packages/core/src/config/schema.rs     # Add image_source, update_check
packages/cli-rust/src/commands/start.rs # Rename flags, add pull logic
packages/cli-rust/src/commands/update.rs # Respect image_source config
packages/cli-rust/src/commands/status.rs # Show image provenance
packages/cli-rust/src/wizard/mod.rs     # Add image source prompt
```

### Pattern 1: Config-Based Defaults with Flag Overrides

**What:** Config sets default behavior, CLI flags override per-invocation
**When to use:** All image acquisition operations
**Example:**
```rust
// In start command
let use_prebuilt = if args.pull_sandbox_image {
    true  // Flag overrides config
} else if args.cached_rebuild_sandbox_image || args.full_rebuild_sandbox_image {
    false  // Rebuild flags mean build
} else {
    config.image_source == "prebuilt"  // Use config default
};
```

### Pattern 2: Wizard State Collection

**What:** WizardState struct collects values before applying to Config
**When to use:** Adding new wizard prompts
**Example (from existing wizard/mod.rs):**
```rust
pub struct WizardState {
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub port: u16,
    pub bind: String,
    // Add: pub image_source: String,
}

impl WizardState {
    pub fn apply_to_config(&self, config: &mut Config) {
        // ... existing fields ...
        // config.image_source = self.image_source.clone();
    }
}
```

### Pattern 3: Image Provenance Tracking

**What:** Store metadata about where image came from
**When to use:** After successful pull or build
**Options:**

1. **Docker image labels** (already have version label)
   - Could add `org.opencode-cloud.source` label during build
   - Pro: Persistent with image
   - Con: Can't add labels to pulled images

2. **Local state file** (recommended)
   - Store in `~/.local/share/opencode-cloud/image-state.json`
   - Track: `{ "version": "1.0.12", "source": "prebuilt", "registry": "ghcr.io", "acquired_at": "..." }`
   - Pro: Can track provenance for any image
   - Con: Can get out of sync if image deleted externally

3. **Infer from image name**
   - If image name contains registry prefix, it was pulled
   - Pro: No extra storage
   - Con: Ambiguous for locally tagged images

**Recommendation:** Option 2 (local state file) because:
- Can track exact registry used
- Survives image re-tagging
- Simple JSON format consistent with config

### Anti-Patterns to Avoid

- **Flag proliferation:** Keep flags verbose and descriptive (`--pull-sandbox-image` not `-p`)
- **Implicit behavior changes:** Always inform user what's happening ("Pulling prebuilt image v1.0.12 from ghcr.io...")
- **Silent failures:** If pull fails 3 times, clearly explain and offer to build instead
- **Version mismatch silence:** Always warn if using mismatched versions

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Registry fallback | Custom retry per registry | Existing `pull_image()` | Already handles GHCR -> Docker Hub fallback |
| Progress reporting | Custom spinners | Existing `ProgressReporter` | Multi-spinner support for layers |
| Interactive prompts | Raw stdin | `dialoguer::Select`, `Confirm` | Handles escape, colors, validation |
| Config serialization | Manual JSON | serde with defaults | `#[serde(default)]` handles migrations |

**Key insight:** This phase is mostly wiring existing components together, not building new ones.

## Common Pitfalls

### Pitfall 1: Flag Mutual Exclusivity

**What goes wrong:** User provides both `--pull-sandbox-image` and `--full-rebuild-sandbox-image`
**Why it happens:** Clap doesn't have built-in mutual exclusivity for non-bool args
**How to avoid:** Check at runtime before proceeding:
```rust
let flags = [args.pull_sandbox_image, args.cached_rebuild_sandbox_image, args.full_rebuild_sandbox_image];
let count = flags.iter().filter(|&&f| f).count();
if count > 1 {
    return Err(anyhow!("Only one of --pull-sandbox-image, --cached-rebuild-sandbox-image, or --full-rebuild-sandbox-image can be specified"));
}
```
**Warning signs:** Tests that use multiple flags should fail

### Pitfall 2: Container Running During Image Change

**What goes wrong:** User uses `--full-rebuild-sandbox-image` while container is running
**Why it happens:** Can't replace image of running container
**How to avoid:** Prompt before proceeding:
```rust
if container_is_running(&client, CONTAINER_NAME).await? {
    let confirm = Confirm::new()
        .with_prompt("Container is running. Stop and rebuild?")
        .default(false)
        .interact()?;
    if !confirm {
        return Err(anyhow!("Aborted. Stop container first with: occ stop"));
    }
    // Proceed to stop and rebuild
}
```
**Warning signs:** "Image not found" errors after rebuild

### Pitfall 3: Pull Failure Without Fallback Offer

**What goes wrong:** Pull fails 3 times and exits with error
**Why it happens:** Not offering build as fallback
**How to avoid:** Catch pull failure and prompt:
```rust
match pull_with_retries(&client, &mut progress).await {
    Ok(image_name) => image_name,
    Err(e) => {
        eprintln!("Failed to pull prebuilt image: {e}");
        let build_instead = Confirm::new()
            .with_prompt("Build from source instead?")
            .default(true)
            .interact()?;
        if build_instead {
            build_image(&client, Some(IMAGE_TAG_DEFAULT), &mut progress, false).await?
        } else {
            return Err(e.into());
        }
    }
}
```
**Warning signs:** Users stuck when registry is down

### Pitfall 4: Version Mismatch Session Warning

**What goes wrong:** User sees mismatch warning every command invocation
**Why it happens:** Not tracking "already warned" state
**How to avoid:** Track warning state per terminal session:
```rust
// Use environment variable for session tracking
const WARN_VAR: &str = "OPENCODE_CLOUD_VERSION_WARNED";

fn should_warn_version_mismatch() -> bool {
    std::env::var(WARN_VAR).is_err()
}

fn mark_warned() {
    std::env::set_var(WARN_VAR, "1");
}
```
Note: Environment variables don't persist across processes. Alternative: file-based timestamp check (warn once per hour).

**Warning signs:** Users complaining about repetitive warnings

### Pitfall 5: Update Command Ignoring Image Source

**What goes wrong:** `occ update` always pulls even when `image_source=build`
**Why it happens:** Forgetting to check config in update command
**How to avoid:** Branch logic in `handle_update()`:
```rust
if config.image_source == "prebuilt" {
    println!("Pulling latest prebuilt image...");
    pull_image(client, Some(IMAGE_TAG_DEFAULT), progress).await?;
} else {
    println!("Rebuilding image from source...");
    build_image(client, Some(IMAGE_TAG_DEFAULT), progress, false).await?;
}
```
**Warning signs:** Build-from-source users getting pulled images

## Code Examples

### Adding Config Fields (from existing patterns)

```rust
// packages/core/src/config/schema.rs

/// Source of Docker image
#[serde(default = "default_image_source")]
pub image_source: String,

/// When to check for updates
#[serde(default = "default_update_check")]
pub update_check: String,

fn default_image_source() -> String {
    "prebuilt".to_string()
}

fn default_update_check() -> String {
    "always".to_string()
}
```

### Adding Wizard Prompt (from existing patterns)

```rust
// In wizard/mod.rs, after quick setup check

let image_source_options = vec![
    "Pull prebuilt image (~2 min, fast)",
    "Build from source (30-60 min, customizable/auditable)",
];

println!("{}", style("Image Source").bold());
println!("Prebuilt images are published automatically and verified.");
println!("Build from source compiles everything locally.");
println!();

let selection = dialoguer::Select::new()
    .with_prompt("How would you like to get the Docker image?")
    .items(&image_source_options)
    .default(0)  // Prebuilt is default
    .interact()
    .map_err(|_| handle_interrupt())?;

let image_source = if selection == 0 { "prebuilt" } else { "build" };
```

### Image Provenance State File

```rust
// packages/core/src/docker/state.rs (new file)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageState {
    pub version: String,
    pub source: String,  // "prebuilt" | "build"
    pub registry: Option<String>,  // "ghcr.io" | "docker.io" | None for build
    pub acquired_at: String,  // ISO8601 timestamp
}

pub fn get_state_path() -> Option<PathBuf> {
    crate::config::paths::get_data_dir()
        .map(|p| p.join("image-state.json"))
}

pub fn save_state(state: &ImageState) -> anyhow::Result<()> {
    let path = get_state_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine state path"))?;
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn load_state() -> Option<ImageState> {
    let path = get_state_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Always build | Pull by default | Phase 15 | Reduces first-run time from 30-60 min to ~2 min |
| Single flag `--rebuild` | Verbose flags `--full-rebuild-sandbox-image` | Phase 15 | Clearer intent |
| No provenance tracking | Local state file | Phase 15 | Users know where image came from |

**Deprecated/outdated:**
- `--cached-rebuild` -> renamed to `--cached-rebuild-sandbox-image`
- `--full-rebuild` -> renamed to `--full-rebuild-sandbox-image`

## Open Questions

1. **Session-based warning suppression implementation**
   - What we know: Environment variables don't persist across processes
   - What's unclear: Best approach for "once per terminal session"
   - Recommendation: Use file-based timestamp (warn once per hour) or accept per-process warning

2. **First-run detection for image source prompt**
   - What we know: Wizard runs on first start if no users configured
   - What's unclear: Should prompt also appear on `occ start` if skipped in wizard?
   - Recommendation: Per CONTEXT.md, show on first `occ start` if not set in wizard (check `image_source` field exists in config)

## Sources

### Primary (HIGH confidence)
- Direct codebase analysis of:
  - `packages/cli-rust/src/commands/start.rs` - Current start flow
  - `packages/cli-rust/src/commands/update.rs` - Current update flow
  - `packages/core/src/docker/image.rs` - Pull/build implementation
  - `packages/core/src/docker/version.rs` - Version detection
  - `packages/core/src/config/schema.rs` - Config structure
  - `packages/cli-rust/src/wizard/mod.rs` - Wizard patterns

### Secondary (MEDIUM confidence)
- `.planning/phases/14-versioning-and-release-automation/14-01-PLAN.md` - Version label implementation
- `.planning/phases/14-versioning-and-release-automation/14-02-PLAN.md` - Version detection design

### Tertiary (LOW confidence)
- None - all findings from direct code analysis

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in use
- Architecture: HIGH - Direct analysis of existing patterns
- Pitfalls: HIGH - Based on code paths and prior phase decisions
- Implementation details: HIGH - Based on existing codebase patterns

**Research date:** 2026-01-24
**Valid until:** 2026-02-24 (stable project, patterns unlikely to change)
