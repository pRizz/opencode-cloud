# Phase 2: Docker Integration - Research

**Researched:** 2026-01-19
**Domain:** Docker container lifecycle management with Rust (Bollard)
**Confidence:** HIGH

## Summary

This phase requires implementing Docker operations in Rust using the Bollard crate, which is the established async Docker client for the Rust ecosystem. Bollard provides a comprehensive API for image building/pulling, container lifecycle management, and volume operations - all with async/await support built on Tokio.

The key technical challenges are:
1. Building images from a Dockerfile with progress streaming
2. Pulling images with layer-by-layer progress feedback
3. Creating containers with proper volume mounts for persistence (session history, project files, configuration)
4. Managing container lifecycle (start/stop/remove) with proper error handling

**Primary recommendation:** Use Bollard 0.18+ with `chrono` and `buildkit` features, indicatif for progress bars, tokio-retry for resilient operations, and named Docker volumes for all persistence requirements.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| [bollard](https://docs.rs/bollard) | 0.18+ | Docker daemon API | Only mature async Docker client for Rust; uses Hyper/Tokio |
| [indicatif](https://docs.rs/indicatif) | 0.17+ | Progress bars | De facto standard for Rust CLI progress (90M+ downloads) |
| [tokio](https://docs.rs/tokio) | 1.43+ | Async runtime | Already in workspace; required by Bollard |
| [futures-util](https://docs.rs/futures-util) | 0.3 | Stream utilities | TryStreamExt for consuming Bollard streams |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| [tar](https://crates.io/crates/tar) | 0.4 | Tar archive creation | Building Docker context for image builds |
| [flate2](https://crates.io/crates/flate2) | 1.0 | Gzip compression | Compressing build context before sending to Docker |
| [tokio-retry](https://docs.rs/tokio-retry) | 0.3 | Async retry logic | Resilient Docker operations (pull, connect) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| bollard | docker-api | docker-api is less maintained; bollard has better async support |
| tokio-retry | backoff | backoff works; tokio-retry integrates better with tokio ecosystem |
| indicatif | linya | indicatif is more mature with better MultiProgress support |

**Installation:**
```bash
# Add to packages/core/Cargo.toml
cargo add bollard --features "chrono,buildkit"
cargo add indicatif --features "tokio,futures"
cargo add futures-util
cargo add tar
cargo add flate2
cargo add tokio-retry
```

## Architecture Patterns

### Recommended Project Structure
```
packages/core/src/
├── docker/              # NEW: Docker operations module
│   ├── mod.rs           # Module exports
│   ├── client.rs        # Docker client wrapper with connection handling
│   ├── image.rs         # Image build/pull operations
│   ├── container.rs     # Container lifecycle operations
│   ├── volume.rs        # Volume management
│   ├── progress.rs      # Progress reporting utilities
│   └── error.rs         # Docker-specific error types
├── config/              # Existing config module
├── singleton/           # Existing singleton module
└── lib.rs               # Add docker module export
```

### Pattern 1: Docker Client Wrapper
**What:** Encapsulate Bollard's Docker client with connection handling and timeout configuration
**When to use:** All Docker operations go through this wrapper
**Example:**
```rust
// Source: https://docs.rs/bollard/latest/bollard/struct.Docker.html
use bollard::Docker;
use bollard::API_DEFAULT_VERSION;

pub struct DockerClient {
    inner: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self, DockerError> {
        // Try local defaults first (handles Unix/Windows automatically)
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| DockerError::Connection(e.to_string()))?;

        Ok(Self { inner: docker })
    }

    pub async fn verify_connection(&self) -> Result<(), DockerError> {
        self.inner.ping().await
            .map_err(|e| DockerError::Connection(e.to_string()))?;
        Ok(())
    }
}
```

### Pattern 2: Streaming Progress with indicatif
**What:** Consume Bollard's async streams while updating progress bars
**When to use:** Image pull/build operations
**Example:**
```rust
// Source: https://docs.rs/indicatif/latest/indicatif/
use bollard::image::CreateImageOptions;
use futures_util::stream::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn pull_image_with_progress(
    docker: &Docker,
    image: &str,
    tag: &str,
) -> Result<(), DockerError> {
    let options = CreateImageOptions {
        from_image: image,
        tag: tag,
        ..Default::default()
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());

    let mut stream = docker.create_image(Some(options), None, None);

    while let Some(info) = stream.try_next().await? {
        if let Some(status) = info.status {
            pb.set_message(status);
        }
        if let Some(progress) = info.progress {
            pb.set_message(format!("{}: {}",
                info.id.unwrap_or_default(), progress));
        }
    }

    pb.finish_with_message("Pull complete");
    Ok(())
}
```

### Pattern 3: Named Volume Mounts
**What:** Configure containers with named volumes for persistent data
**When to use:** Container creation for session history, project files, config
**Example:**
```rust
// Source: https://docs.rs/bollard/latest/bollard/service/struct.Mount.html
use bollard::service::{Mount, MountTypeEnum, HostConfig};
use bollard::container::{Config, CreateContainerOptions};

pub async fn create_container_with_volumes(
    docker: &Docker,
    name: &str,
) -> Result<String, DockerError> {
    let mounts = vec![
        Mount {
            target: Some("/home/opencode/.opencode".to_string()),
            source: Some("opencode-session".to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
        Mount {
            target: Some("/workspace".to_string()),
            source: Some("opencode-projects".to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
        Mount {
            target: Some("/home/opencode/.config".to_string()),
            source: Some("opencode-config".to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
    ];

    let host_config = HostConfig {
        mounts: Some(mounts),
        ..Default::default()
    };

    let config = Config {
        image: Some("ghcr.io/prizz/opencode-cloud:latest"),
        host_config: Some(host_config),
        ..Default::default()
    };

    let response = docker.create_container(
        Some(CreateContainerOptions { name, platform: None }),
        config,
    ).await?;

    Ok(response.id)
}
```

### Pattern 4: Building Images from Dockerfile
**What:** Create tar archive of build context and stream to Docker daemon
**When to use:** Building the opencode image locally
**Example:**
```rust
// Source: https://github.com/fussybeaver/bollard examples
use bollard::query_parameters::BuildImageOptionsBuilder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder as TarBuilder;

pub async fn build_image(
    docker: &Docker,
    dockerfile_content: &str,
    tag: &str,
) -> Result<(), DockerError> {
    // Create tar archive with Dockerfile
    let mut header = tar::Header::new_gnu();
    header.set_path("Dockerfile")?;
    header.set_size(dockerfile_content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();

    let mut tar = TarBuilder::new(Vec::new());
    tar.append(&header, dockerfile_content.as_bytes())?;
    let uncompressed = tar.into_inner()?;

    // Compress with gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, &uncompressed)?;
    let compressed = encoder.finish()?;

    // Build image
    let options = BuildImageOptionsBuilder::default()
        .t(tag)
        .dockerfile("Dockerfile")
        .rm(true)
        .build();

    let mut stream = docker.build_image(
        options,
        None,
        Some(http_body_util::Either::Left(
            http_body_util::Full::new(compressed.into())
        )),
    );

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                if let Some(stream) = info.stream {
                    print!("{}", stream);
                }
            }
            Err(e) => return Err(DockerError::Build(e.to_string())),
        }
    }

    Ok(())
}
```

### Anti-Patterns to Avoid
- **Blocking on streams:** Never use `.collect()` on Bollard streams without progress feedback - operations can take minutes
- **Ignoring connection errors:** Always verify Docker daemon connection before operations
- **Hardcoded socket paths:** Use `connect_with_local_defaults()` for cross-platform support
- **Missing timeout configuration:** Always set reasonable timeouts (default 2 min may be too short for builds)

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Progress bars | Custom terminal output | indicatif | Handles terminal width, unicode, multi-threaded updates |
| Tar archives | Manual byte manipulation | tar crate | Proper header formatting, path handling, compression |
| Retry logic | Simple loop with sleep | tokio-retry | Exponential backoff, jitter, configurable strategies |
| Gzip compression | Raw flate2 streams | flate2::write::GzEncoder | Handles buffering, proper finalization |
| Docker connection | Manual socket handling | Bollard | Handles Unix/Windows, SSL, version negotiation |

**Key insight:** Docker operations involve many edge cases (partial downloads, interrupted builds, permission errors). Bollard and its ecosystem handle these; custom solutions will miss edge cases.

## Common Pitfalls

### Pitfall 1: Build Context Not Found
**What goes wrong:** "Cannot locate specified Dockerfile: Dockerfile" error during image build
**Why it happens:** Dockerfile path in tar archive doesn't match `dockerfile` option
**How to avoid:** Ensure tar header path matches exactly: `header.set_path("Dockerfile")` and `dockerfile("Dockerfile")`
**Warning signs:** Build fails immediately without attempting any layers

### Pitfall 2: Stream Consumption Without Error Handling
**What goes wrong:** Errors silently swallowed, operation appears to complete but fails
**Why it happens:** Using `.next().await` without matching on Result
**How to avoid:** Always use `try_next()` or match on `Result`:
```rust
while let Some(result) = stream.try_next().await? {
    // Handle success case
}
```
**Warning signs:** Operations complete quickly without expected output

### Pitfall 3: Volume Permissions
**What goes wrong:** Container can't write to mounted volumes; "permission denied" errors
**Why it happens:** Volume created by root, container runs as non-root user
**How to avoid:** Either:
1. Create volumes with correct ownership in entrypoint script
2. Use `volume_options` to specify correct driver options
3. Ensure Dockerfile sets correct permissions on target directories
**Warning signs:** Container starts but opencode can't save session/config

### Pitfall 4: Named Pipe Issues on Windows
**What goes wrong:** Connection fails on Windows with cryptic errors
**Why it happens:** Windows named pipe path differs from Unix socket
**How to avoid:** Use `connect_with_local_defaults()` which auto-detects platform
**Warning signs:** Works on macOS/Linux, fails on Windows (deferred to v2 but worth noting)

### Pitfall 5: Image Pull Layer Parallelism
**What goes wrong:** Progress reporting shows confusing interleaved layer status
**Why it happens:** Docker pulls multiple layers in parallel; each layer sends separate progress
**How to avoid:** Use `MultiProgress` from indicatif with one progress bar per layer ID:
```rust
use indicatif::MultiProgress;
use std::collections::HashMap;

let multi = MultiProgress::new();
let mut layers: HashMap<String, ProgressBar> = HashMap::new();

while let Some(info) = stream.try_next().await? {
    if let Some(id) = &info.id {
        let pb = layers.entry(id.clone())
            .or_insert_with(|| multi.add(ProgressBar::new(100)));
        // Update pb based on info.progress_detail
    }
}
```
**Warning signs:** Progress bar flickering, confusing output during pulls

### Pitfall 6: Timeout During Large Image Builds
**What goes wrong:** Build times out before completing
**Why it happens:** Default Bollard timeout is 2 minutes; image builds can take 10+ minutes
**How to avoid:** Increase timeout when creating Docker client:
```rust
let docker = Docker::connect_with_unix(
    "/var/run/docker.sock",
    600, // 10 minute timeout
    API_DEFAULT_VERSION,
)?;
```
**Warning signs:** Builds fail after exactly 2 minutes

## Code Examples

Verified patterns from official sources:

### Container Lifecycle (Start/Stop/Remove)
```rust
// Source: https://docs.rs/bollard/latest/bollard/struct.Docker.html
use bollard::container::{StartContainerOptions, StopContainerOptions, RemoveContainerOptions};

impl DockerClient {
    pub async fn start_container(&self, name: &str) -> Result<(), DockerError> {
        self.inner.start_container(name, None::<StartContainerOptions<String>>).await?;
        Ok(())
    }

    pub async fn stop_container(&self, name: &str, timeout_secs: i64) -> Result<(), DockerError> {
        let options = StopContainerOptions { t: timeout_secs };
        self.inner.stop_container(name, Some(options)).await?;
        Ok(())
    }

    pub async fn remove_container(&self, name: &str, force: bool) -> Result<(), DockerError> {
        let options = RemoveContainerOptions {
            force,
            v: true, // Remove associated volumes
            ..Default::default()
        };
        self.inner.remove_container(name, Some(options)).await?;
        Ok(())
    }
}
```

### Volume Creation
```rust
// Source: https://docs.rs/bollard/latest/bollard/volume/
use bollard::volume::CreateVolumeOptions;

pub async fn ensure_volumes_exist(docker: &Docker) -> Result<(), DockerError> {
    let volume_names = ["opencode-session", "opencode-projects", "opencode-config"];

    for name in volume_names {
        let options = CreateVolumeOptions {
            name: name.to_string(),
            ..Default::default()
        };
        // create_volume is idempotent - returns existing volume if it exists
        docker.create_volume(options).await?;
    }

    Ok(())
}
```

### Retry Pattern for Resilient Operations
```rust
// Source: https://docs.rs/tokio-retry/latest/tokio_retry/
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

pub async fn pull_image_with_retry(
    docker: &Docker,
    image: &str,
) -> Result<(), DockerError> {
    let retry_strategy = ExponentialBackoff::from_millis(100)
        .map(jitter)
        .take(3); // Max 3 retries

    Retry::spawn(retry_strategy, || async {
        pull_image(docker, image).await
    }).await
}
```

### Container Logs Streaming
```rust
// Source: https://docs.rs/bollard/latest/bollard/container/struct.LogsOptions.html
use bollard::container::{LogsOptions, LogOutput};
use futures_util::stream::StreamExt;

pub async fn stream_container_logs(
    docker: &Docker,
    container_name: &str,
) -> impl Stream<Item = Result<String, DockerError>> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow: true,
        ..Default::default()
    };

    docker.logs(container_name, Some(options))
        .map(|result| {
            result
                .map(|output| match output {
                    LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                    _ => String::new(),
                })
                .map_err(|e| DockerError::Logs(e.to_string()))
        })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `bollard::image::BuildImageOptions` | `bollard::query_parameters::BuildImageOptionsBuilder` | v0.19.0 | Must use builder pattern |
| Manual tar creation | tar + flate2 crates | - | Standard Rust approach |
| Blocking Docker clients | Async (Bollard + Tokio) | ~2020 | All operations must be async |
| Docker API 1.40 | Docker API 1.52 | 2025 | New mount types, better streaming |

**Deprecated/outdated:**
- `bollard::image::BuildImageOptions` - deprecated since 0.19.0, use `query_parameters::BuildImageOptionsBuilder`
- `bollard::container::Config` - some fields moved to `ContainerCreateBody`

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal volume driver for cross-platform**
   - What we know: Named volumes work on all platforms
   - What's unclear: Whether `local` driver is sufficient or if we need platform-specific options
   - Recommendation: Use default `local` driver; revisit if issues arise

2. **BuildKit vs legacy builder**
   - What we know: BuildKit is faster and has better caching
   - What's unclear: Whether we need BuildKit features or if legacy builder suffices
   - Recommendation: Start with legacy builder (simpler); add BuildKit if needed for performance

3. **Exact progress bar format for layer pulls**
   - What we know: Docker sends layer progress with current/total bytes
   - What's unclear: Best UX for displaying 5-10 parallel layer downloads
   - Recommendation: Use MultiProgress; experiment with condensed vs expanded view

## Sources

### Primary (HIGH confidence)
- [Bollard docs.rs](https://docs.rs/bollard/latest/bollard/) - API reference
- [Bollard GitHub](https://github.com/fussybeaver/bollard) - Examples, README
- [indicatif docs.rs](https://docs.rs/indicatif/latest/indicatif/) - Progress bar API
- [Docker Official Docs](https://docs.docker.com/engine/storage/volumes/) - Volume best practices

### Secondary (MEDIUM confidence)
- [Rust Cookbook - Tarballs](https://rust-lang-nursery.github.io/rust-cookbook/compression/tar.html) - Tar/gzip patterns
- [tokio-retry docs](https://docs.rs/tokio-retry) - Retry strategies
- Various blog posts on Docker healthchecks and best practices (cross-verified with official docs)

### Tertiary (LOW confidence)
- Stack Overflow / Rust forum discussions - Specific edge case solutions (need validation)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Bollard is the clear choice for async Rust Docker
- Architecture: HIGH - Patterns are well-established in Bollard examples
- Pitfalls: MEDIUM - Some gathered from community reports, may miss edge cases
- Volume persistence: MEDIUM - Named volumes are standard, but exact mount points for opencode need validation

**Research date:** 2026-01-19
**Valid until:** 60 days (Bollard is stable; Docker API changes slowly)
