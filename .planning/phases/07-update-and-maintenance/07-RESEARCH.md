# Phase 7: Update and Maintenance - Research

**Researched:** 2026-01-21
**Domain:** Docker container updates, health monitoring, config validation
**Confidence:** HIGH

## Summary

This phase covers three main capabilities: (1) an `occ update` command to pull the latest Docker image and recreate the container, (2) a `/health` endpoint for monitoring tools like AWS ALB, and (3) config validation on startup with clear error messages.

The standard approach uses bollard's existing image pull/tag APIs for updates, proxies OpenCode's built-in `/global/health` endpoint for health checks, and adds explicit validation methods to the existing Config struct. Update notifications can be implemented with the `update-informer` crate for non-blocking version checks.

**Primary recommendation:** Use Docker image digest comparison via bollard's `inspect_registry_image` to detect new versions; proxy OpenCode's native health endpoint; add a `Config::validate()` method returning a list of validation errors with fix commands.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bollard | 0.18+ | Docker API (image pull, tag, inspect_registry_image) | Already in use; has all needed APIs |
| update-informer | 1.1+ | Check for new CLI versions | Purpose-built for Rust CLIs, supports caching |
| serde_json | 1.0 | Config validation error context | Already in use |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | 0.4 | Timestamp management for update checks | Already in use; needed for last-check tracking |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| update-informer | tiny_update_notifier | update-informer has more registry support and better caching |
| bollard inspect_registry_image | docker manifest inspect CLI | bollard API is native Rust, no subprocess needed |

**Installation:**
```bash
# update-informer already supports GitHub; no new deps needed for Docker image checks
# Just add update-informer if CLI version notifications are desired
cargo add update-informer
```

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── docker/
│   ├── update.rs        # New: image digest comparison, pull, rollback
│   └── health.rs        # New: health check via container exec
├── config/
│   ├── schema.rs        # Existing: add validate() method
│   └── validation.rs    # New: validation rules and error formatting
└── update/
    └── check.rs         # New: version comparison, notification caching

packages/cli-rust/src/
├── commands/
│   └── update.rs        # New: occ update command
```

### Pattern 1: Image Update with Rollback Support
**What:** Tag current image as "previous" before pulling new, enable rollback
**When to use:** Every update operation
**Example:**
```rust
// Source: Bollard tag_image API (https://docs.rs/bollard/latest/bollard/struct.Docker.html)
use bollard::query_parameters::TagImageOptionsBuilder;

pub async fn update_with_rollback(
    client: &DockerClient,
) -> Result<UpdateResult, DockerError> {
    let current_image = format!("{}:{}", IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT);

    // Step 1: Tag current as "previous" for rollback
    let tag_opts = TagImageOptionsBuilder::default()
        .repo(IMAGE_NAME_GHCR)
        .tag("previous")
        .build();
    client.inner().tag_image(&current_image, Some(tag_opts)).await?;

    // Step 2: Pull latest
    let mut progress = ProgressReporter::with_context("Pulling latest image");
    pull_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress).await?;

    // Step 3: Stop, remove, recreate container
    stop_service(client, true).await?;
    setup_and_start(client, None, None, None).await?;

    Ok(UpdateResult::Success)
}

pub async fn rollback(client: &DockerClient) -> Result<(), DockerError> {
    let previous_image = format!("{}:previous", IMAGE_NAME_GHCR);

    // Re-tag "previous" as "latest"
    let tag_opts = TagImageOptionsBuilder::default()
        .repo(IMAGE_NAME_GHCR)
        .tag(IMAGE_TAG_DEFAULT)
        .build();
    client.inner().tag_image(&previous_image, Some(tag_opts)).await?;

    // Recreate container from restored image
    stop_service(client, true).await?;
    setup_and_start(client, None, None, None).await?;

    Ok(())
}
```

### Pattern 2: Health Check via OpenCode's Native Endpoint
**What:** Proxy OpenCode's built-in `/global/health` endpoint
**When to use:** Health check queries
**Example:**
```rust
// Source: OpenCode docs (https://opencode.ai/docs/server/)
// OpenCode's /global/health returns: { healthy: true, version: "..." }

use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub version: String,
}

pub async fn check_health(port: u16) -> Result<HealthResponse, HealthError> {
    let url = format!("http://127.0.0.1:{}/global/health", port);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        Ok(response.json::<HealthResponse>().await?)
    } else {
        Err(HealthError::Unhealthy(response.status().as_u16()))
    }
}

// For external monitoring, extend with container stats
#[derive(Debug, Serialize)]
pub struct ExtendedHealthResponse {
    pub healthy: bool,
    pub version: String,
    pub container_state: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: Option<u64>,
}
```

### Pattern 3: Config Validation with Actionable Errors
**What:** Validate config and return errors with fix commands
**When to use:** On `config set` and on service start
**Example:**
```rust
// Pattern: Single error at a time with fix guidance
#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub fix_command: String,
}

impl Config {
    pub fn validate(&self) -> Result<Vec<ValidationWarning>, ValidationError> {
        let mut warnings = Vec::new();

        // Port validation
        if self.opencode_web_port < 1024 {
            return Err(ValidationError {
                field: "opencode_web_port".to_string(),
                message: "Port must be >= 1024 (non-privileged)".to_string(),
                fix_command: "occ config set opencode_web_port 3000".to_string(),
            });
        }

        // Bind address validation
        if let Err(e) = validate_bind_address(&self.bind_address) {
            return Err(ValidationError {
                field: "bind_address".to_string(),
                message: e,
                fix_command: "occ config set bind_address 127.0.0.1".to_string(),
            });
        }

        // Security warning (non-fatal by default)
        if self.is_network_exposed() && self.users.is_empty()
           && !self.allow_unauthenticated_network {
            warnings.push(ValidationWarning {
                field: "security".to_string(),
                message: "Network exposed without authentication".to_string(),
                fix_command: "occ user add".to_string(),
            });
        }

        Ok(warnings)
    }
}
```

### Pattern 4: Update Notification with Caching
**What:** Background check for new versions, cached to avoid slowdown
**When to use:** At end of CLI command execution
**Example:**
```rust
// Source: update-informer (https://lib.rs/crates/update-informer)
use update_informer::{registry, Check};
use std::time::Duration;

const CHECK_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60); // weekly

pub fn check_for_cli_update() -> Option<String> {
    let informer = update_informer::new(registry::Crates, "opencode-cloud", env!("CARGO_PKG_VERSION"))
        .interval(CHECK_INTERVAL);

    informer.check_version().ok().flatten().map(|v| v.to_string())
}

pub fn print_update_notification_if_available() {
    if let Some(new_version) = check_for_cli_update() {
        eprintln!();
        eprintln!(
            "{} A new version of opencode-cloud is available: {} -> {}",
            style("Update:").cyan(),
            env!("CARGO_PKG_VERSION"),
            style(&new_version).green()
        );
        eprintln!("  Run: cargo install opencode-cloud");
    }
}
```

### Anti-Patterns to Avoid
- **Blocking update checks in main flow:** Update checks should be non-blocking and cached; never slow down primary commands
- **Losing previous image before confirming new works:** Always tag "previous" before pulling new image
- **Generic validation errors:** Always include the field name, why it's wrong, and an exact command to fix it
- **Health endpoint requiring authentication:** Load balancers need unauthenticated access to `/health`

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI update notifications | Custom registry checker | update-informer | Handles caching, multiple registries, timeout |
| Docker image version comparison | Parsing tags manually | bollard inspect_registry_image | Returns digest, handles auth, retries |
| Health endpoint | Custom HTTP server | OpenCode's /global/health | Already exists, includes version info |
| Config validation errors | String concatenation | Structured ValidationError type | Enables programmatic handling, consistent UX |

**Key insight:** OpenCode already has a health endpoint at `/global/health` returning JSON with healthy status and version - we should proxy this rather than implement a separate health check.

## Common Pitfalls

### Pitfall 1: Not Preserving Data During Update
**What goes wrong:** User loses configuration or session data after update
**Why it happens:** Container is recreated but volumes are not properly remounted
**How to avoid:**
- Use named volumes (already in place: opencode-cloud-session, opencode-cloud-projects, opencode-cloud-config)
- Recreate container with same volume mounts
- Don't remove volumes during update (remove_container(force=false, v=false))
**Warning signs:** User reports lost sessions after update

### Pitfall 2: Update During Active Session
**What goes wrong:** User loses in-progress AI conversation
**Why it happens:** Stop command terminates container mid-session
**How to avoid:**
- Warn user that update will restart the service
- Show container uptime in status before update
- Document that update causes brief downtime (per CONTEXT.md decision)
**Warning signs:** Angry users who lost work

### Pitfall 3: Rollback Image Not Available
**What goes wrong:** --rollback fails because "previous" tag doesn't exist
**Why it happens:** First update hasn't been done yet, or manual image cleanup
**How to avoid:**
- Check if "previous" tag exists before attempting rollback
- Return clear error: "No previous version available for rollback"
**Warning signs:** Rollback command fails with cryptic Docker error

### Pitfall 4: Health Check False Positives
**What goes wrong:** Health returns 200 but service isn't actually functional
**Why it happens:** Container running but OpenCode process hasn't started yet
**How to avoid:**
- Use OpenCode's /global/health which checks actual service readiness
- Return 503 if health check times out or returns unhealthy
**Warning signs:** Load balancer sends traffic to unready instance

### Pitfall 5: Blocking Update Notification Checks
**What goes wrong:** `occ status` takes 5+ seconds to complete
**Why it happens:** Version check blocks waiting for network response
**How to avoid:**
- Use update-informer's caching (default 24h, we use weekly)
- Make check non-blocking (check cache first, async fetch in background)
- Respect timeout (5s default is appropriate)
**Warning signs:** Commands slow when offline or with poor network

## Code Examples

Verified patterns from official sources:

### Docker Image Tagging (for Rollback)
```rust
// Source: Bollard docs (https://docs.rs/bollard/latest/bollard/struct.Docker.html)
use bollard::query_parameters::TagImageOptionsBuilder;

async fn tag_for_rollback(client: &DockerClient, image: &str) -> Result<(), DockerError> {
    let options = TagImageOptionsBuilder::default()
        .repo(IMAGE_NAME_GHCR)
        .tag("previous")
        .build();

    client.inner().tag_image(image, Some(options)).await
        .map_err(|e| DockerError::Tag(format!("Failed to tag image: {}", e)))
}
```

### Check for New Image Version Available
```rust
// Source: Bollard docs - inspect_registry_image
async fn newer_version_available(
    client: &DockerClient,
    current_digest: Option<&str>,
) -> Result<bool, DockerError> {
    // Get remote digest from registry
    let remote_info = client.inner()
        .inspect_registry_image(&format!("{}:{}", IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT), None)
        .await
        .map_err(|e| DockerError::Registry(format!("Failed to check registry: {}", e)))?;

    let remote_digest = remote_info.descriptor
        .and_then(|d| d.digest);

    match (current_digest, remote_digest) {
        (Some(current), Some(remote)) => Ok(current != remote),
        _ => Ok(true), // Assume update available if can't compare
    }
}
```

### Health Check Response Format (AWS ALB Compatible)
```rust
// Source: AWS ALB docs - health checks expect 200 for healthy, 503 for unhealthy
// Response body should include Content-Length header
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub healthy: bool,
    pub version: String,
    pub container_state: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage_mb: Option<u64>,
}

// When returning HTTP response:
// - 200 OK with JSON body for healthy
// - 503 Service Unavailable for unhealthy
// AWS ALB default success codes: 200-299
```

### Config Validation Error Display
```rust
// Pattern: Stop at first error, show fix command
fn display_validation_error(error: &ValidationError) {
    eprintln!();
    eprintln!("{} Configuration error", style("Error:").red().bold());
    eprintln!();
    eprintln!("  Field:   {}", style(&error.field).yellow());
    eprintln!("  Problem: {}", error.message);
    eprintln!();
    eprintln!("To fix, run:");
    eprintln!("  {}", style(&error.fix_command).cyan());
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Docker image tags for versioning | Digests for immutable identification | Always best practice | Reliable update detection |
| Custom health servers | Proxy existing service health endpoints | N/A | Less code, single source of truth |
| Inline config validation | Structured ValidationError types | N/A | Better UX, programmatic handling |
| Synchronous version checks | Cached async checks | N/A | No CLI slowdown |

**Deprecated/outdated:**
- None identified - all technologies are current

## Open Questions

Things that couldn't be fully resolved:

1. **OpenCode version in container**
   - What we know: OpenCode server exposes version at /global/health
   - What's unclear: How to get version from image without starting container (for comparing before update)
   - Recommendation: Start container briefly to query version, or accept that we check digest instead of semantic version

2. **GHCR authentication for inspect_registry_image**
   - What we know: bollard supports DockerCredentials for authenticated registry access
   - What's unclear: Whether GHCR public images require authentication for manifest inspection
   - Recommendation: Try unauthenticated first, fallback to credential prompt if needed

3. **Update notification frequency configuration**
   - What we know: update-informer defaults to 24h, we planned for weekly
   - What's unclear: Should this be user-configurable in config.json?
   - Recommendation: Start with hardcoded weekly, add config option in future if requested

## Sources

### Primary (HIGH confidence)
- Bollard 0.18 docs - tag_image, inspect_registry_image APIs (https://docs.rs/bollard/latest/bollard/)
- OpenCode server docs - /global/health endpoint (https://opencode.ai/docs/server/)
- AWS ALB health check docs - HTTP status code requirements (https://docs.aws.amazon.com/elasticloadbalancing/latest/application/target-group-health-checks.html)
- update-informer crate docs - caching, interval configuration (https://lib.rs/crates/update-informer)

### Secondary (MEDIUM confidence)
- Docker Registry HTTP API - digest comparison (https://docs.docker.com/registry/spec/api/)
- Skopeo project - remote image inspection patterns (https://github.com/containers/skopeo)

### Tertiary (LOW confidence)
- Forum discussions on update notification UX patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - using existing crate (bollard) with documented APIs
- Architecture: HIGH - follows existing codebase patterns exactly
- Pitfalls: HIGH - based on explicit decisions in CONTEXT.md and documented Docker behaviors

**Research date:** 2026-01-21
**Valid until:** 2026-02-21 (30 days - stable technology domain)
