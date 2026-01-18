# Architecture Patterns

**Project:** opencode-cloud-service
**Domain:** Cross-platform CLI installer for containerized AI coding agent service
**Researched:** 2026-01-18
**Confidence:** MEDIUM-HIGH

## Executive Summary

This architecture document describes a monorepo structure for a cross-platform CLI installer that deploys opencode as a persistent cloud service. The system consists of five major components: CLI entry points (npm/npx and cargo), Docker management, service installation, configuration management, and shared Docker assets. The design follows the cloudflared pattern for cross-platform service installation while leveraging Bollard (Rust) and Dockerode (TypeScript) for programmatic Docker control.

---

## Recommended Architecture

```
opencode-cloud-service/
|
|-- packages/
|   |-- cli-node/              # npm/npx entry point (TypeScript)
|   |-- cli-rust/              # cargo entry point (Rust)
|   |-- shared-types/          # Shared TypeScript types
|   |
|-- docker/
|   |-- Dockerfile             # Main opencode container
|   |-- docker-compose.yml     # Orchestration template
|   |-- templates/             # Platform-specific compose overrides
|   |
|-- services/
|   |-- linux/                 # systemd unit files
|   |-- macos/                 # launchd plist templates
|   |-- windows/               # Windows Service definitions
|   |
|-- turbo.json                 # Turborepo configuration
|-- pnpm-workspace.yaml        # pnpm workspaces config
|-- Cargo.toml                 # Rust workspace root
|-- package.json               # Root package.json
```

### High-Level Component Diagram

```
+-------------------+     +-------------------+
|   npm/npx CLI     |     |    cargo CLI      |
|   (TypeScript)    |     |      (Rust)       |
+--------+----------+     +--------+----------+
         |                         |
         |   Shared Interface      |
         +------------+------------+
                      |
         +------------v------------+
         |   Docker Management     |
         |  Dockerode / Bollard    |
         +------------+------------+
                      |
         +------------v------------+
         |   Service Installation  |
         | systemd/launchd/SCM     |
         +------------+------------+
                      |
         +------------v------------+
         |   Config Management     |
         |   JSON persistence      |
         +-------------------------+
```

---

## Component Boundaries

| Component | Responsibility | Technology | Communicates With |
|-----------|---------------|------------|-------------------|
| **CLI Node** | npm/npx entry point, user interaction, command parsing | TypeScript, Commander.js or yargs | Docker Management, Config Management, Service Installation |
| **CLI Rust** | cargo entry point, same features as Node CLI | Rust, clap | Docker Management, Config Management, Service Installation |
| **Docker Management** | Container lifecycle, image building, compose orchestration | Dockerode (Node), Bollard (Rust) | Docker daemon via socket/API |
| **Service Installation** | Register/unregister as system service | Platform-specific APIs | OS service managers (systemd, launchd, SCM) |
| **Config Management** | Persist user configuration, secrets handling | JSON files in XDG-compliant paths | File system |
| **Docker Assets** | Dockerfile, compose files, templates | Docker, docker-compose | Used by Docker Management |

### Component Detail

#### 1. CLI Entry Points

Both CLIs expose identical commands with consistent behavior:

```
opencode-cloud-service install    # Install as system service
opencode-cloud-service uninstall  # Remove system service
opencode-cloud-service start      # Start the service
opencode-cloud-service stop       # Stop the service
opencode-cloud-service status     # Show service status
opencode-cloud-service config     # Manage configuration
opencode-cloud-service logs       # View service logs
opencode-cloud-service update     # Update container image
```

**Boundary:** CLIs are thin wrappers that delegate to shared logic modules. They handle:
- Argument parsing
- User prompts and output formatting
- Error presentation
- Platform detection

**Does NOT handle:** Docker operations, service registration, config I/O.

#### 2. Docker Management

Manages all Docker interactions programmatically.

| Operation | Description | Library Method |
|-----------|-------------|----------------|
| Check Docker | Verify Docker daemon is running | `docker.ping()` |
| Pull Image | Download opencode image | `docker.pull()` |
| Create Container | Instantiate from image | `docker.createContainer()` |
| Start/Stop | Lifecycle management | `container.start()` / `container.stop()` |
| Logs | Stream container output | `container.logs()` |
| Health Check | Monitor container status | `container.inspect()` |

**Connection Methods:**
- **Linux/macOS:** Unix socket (`/var/run/docker.sock`)
- **Windows:** Named pipe (`//./pipe/docker_engine`)
- **Remote:** TCP with optional TLS (via `DOCKER_HOST`)

**Boundary:** This component owns ALL Docker interactions. No other component directly calls Docker APIs.

#### 3. Service Installation

Platform-specific service registration following the [cloudflared pattern](https://deepwiki.com/cloudflare/cloudflared/2.3-service-management-commands).

| Platform | Service Manager | Config Location | Implementation |
|----------|-----------------|-----------------|----------------|
| Linux | systemd | `/etc/systemd/system/` | `.service` unit file |
| Linux (legacy) | SysV | `/etc/init.d/` | init script |
| macOS | launchd | `/Library/LaunchDaemons/` | `.plist` file |
| Windows | SCM | Registry + service binary | Windows Service API |

**Service Lifecycle:**

```
install:
  1. Detect platform and service manager
  2. Generate service definition from template
  3. Write to appropriate system location
  4. Register with service manager
  5. Optionally enable auto-start

uninstall:
  1. Stop running service
  2. Disable auto-start
  3. Unregister from service manager
  4. Remove service definition file
```

**Boundary:** This component handles OS service manager interactions ONLY. It does not manage Docker directly; it configures the service to invoke the Docker management component.

#### 4. Configuration Management

Cross-platform JSON configuration persistence using XDG-compliant paths.

**Path Resolution:**

| Platform | Config Directory | Data Directory |
|----------|------------------|----------------|
| Linux | `~/.config/opencode-cloud-service/` | `~/.local/share/opencode-cloud-service/` |
| macOS | `~/Library/Preferences/opencode-cloud-service/` | `~/Library/Application Support/opencode-cloud-service/` |
| Windows | `%APPDATA%\opencode-cloud-service\Config\` | `%LOCALAPPDATA%\opencode-cloud-service\Data\` |

**Libraries:**
- **TypeScript:** [env-paths](https://github.com/sindresorhus/env-paths) (sindresorhus)
- **Rust:** [directories](https://crates.io/crates/directories) or [cross-xdg](https://crates.io/crates/cross-xdg)

**Config Schema (example):**

```json
{
  "version": "1.0.0",
  "container": {
    "image": "opencode:latest",
    "port": 8080,
    "volumes": ["/path/to/workspace:/workspace"]
  },
  "service": {
    "autoStart": true,
    "restartPolicy": "always"
  },
  "auth": {
    "apiKeyRef": "env:OPENCODE_API_KEY"
  }
}
```

**Boundary:** Config management handles file I/O, schema validation, and migration. It does NOT interpret config values; consumers (Docker Management, Service Installation) use the parsed config.

#### 5. Docker Assets

Shared Docker configuration consumed by both CLIs.

```
docker/
|-- Dockerfile              # Multi-stage build for opencode
|-- docker-compose.yml      # Base compose configuration
|-- templates/
    |-- docker-compose.linux.yml
    |-- docker-compose.macos.yml
    |-- docker-compose.windows.yml
```

**Boundary:** These are static assets. Docker Management reads and processes them; they are not executable components.

---

## Data Flow

### Installation Flow

```
User runs: npx opencode-cloud-service install

1. CLI: Parse arguments, detect platform
        |
2. CLI: Prompt for configuration (if interactive)
        |
3. Config: Write config.json to XDG path
        |
4. Docker: Check Docker daemon availability
        |
5. Docker: Pull/build opencode image
        |
6. Docker: Verify container can start
        |
7. Service: Generate platform-specific service file
        |
8. Service: Register with OS service manager
        |
9. Service: Start service
        |
10. CLI: Report success with status
```

### Runtime Flow (Service Running)

```
OS Service Manager
        |
        v
Service Definition (systemd/launchd/SCM)
        |
        v
Docker daemon socket/pipe
        |
        v
opencode container
        |
        v
[Exposes port / socket for client connections]
```

### Configuration Update Flow

```
User runs: npx opencode-cloud-service config set port 9090

1. CLI: Parse config subcommand
        |
2. Config: Read existing config.json
        |
3. Config: Validate new value against schema
        |
4. Config: Write updated config.json
        |
5. Service: If running, prompt to restart
        |
6. Docker: Recreate container with new config (if restart confirmed)
```

---

## Patterns to Follow

### Pattern 1: Thin CLI, Thick Core

**What:** CLIs should be thin wrappers delegating to shared business logic.

**Why:** Ensures feature parity between npm and cargo CLIs. Reduces duplication.

**Implementation:**

For TypeScript, core logic lives in shared modules:
```typescript
// packages/cli-node/src/commands/install.ts
import { install } from '@opencode-cloud-service/core';

export async function installCommand(options: InstallOptions) {
  const spinner = ora('Installing...').start();
  try {
    await install(options);
    spinner.succeed('Installed successfully');
  } catch (err) {
    spinner.fail(err.message);
    process.exit(1);
  }
}
```

For Rust, core logic in a library crate:
```rust
// packages/cli-rust/src/main.rs
use opencode_core::install;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(e) = install(args.into()).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

### Pattern 2: Platform Abstraction Layer

**What:** Abstract platform-specific operations behind a common interface.

**Why:** Clean separation between business logic and OS-specific code.

**Implementation:**

```typescript
// Shared interface
interface ServiceManager {
  install(config: ServiceConfig): Promise<void>;
  uninstall(name: string): Promise<void>;
  start(name: string): Promise<void>;
  stop(name: string): Promise<void>;
  status(name: string): Promise<ServiceStatus>;
}

// Platform implementations
class SystemdManager implements ServiceManager { /* ... */ }
class LaunchdManager implements ServiceManager { /* ... */ }
class WindowsSCMManager implements ServiceManager { /* ... */ }

// Factory
function getServiceManager(): ServiceManager {
  switch (process.platform) {
    case 'linux': return new SystemdManager();
    case 'darwin': return new LaunchdManager();
    case 'win32': return new WindowsSCMManager();
    default: throw new Error(`Unsupported platform: ${process.platform}`);
  }
}
```

### Pattern 3: Docker Socket Detection

**What:** Auto-detect Docker daemon connection method.

**Why:** Works across all platforms without user configuration.

**Implementation:**

```typescript
function getDockerConnection(): DockerConnectionOptions {
  if (process.platform === 'win32') {
    return { socketPath: '//./pipe/docker_engine' };
  }

  // Check for custom DOCKER_HOST
  if (process.env.DOCKER_HOST) {
    return parseDockerHost(process.env.DOCKER_HOST);
  }

  // Default Unix socket
  return { socketPath: '/var/run/docker.sock' };
}
```

### Pattern 4: Graceful Degradation for SysV

**What:** Fall back to SysV init scripts on Linux systems without systemd.

**Why:** Supports older/minimal Linux distributions.

**Detection:**
```typescript
function hasSystemd(): boolean {
  try {
    fs.accessSync('/run/systemd/system', fs.constants.R_OK);
    return true;
  } catch {
    return false;
  }
}
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Direct Docker CLI Shelling

**What:** Calling `docker` CLI via subprocess instead of using SDK.

**Why Bad:**
- Parsing CLI output is fragile
- Requires Docker CLI to be installed (not just daemon)
- Error handling is inconsistent
- No type safety

**Instead:** Use Dockerode (Node) or Bollard (Rust) to communicate with Docker daemon directly.

### Anti-Pattern 2: Hardcoded Paths

**What:** Hardcoding config/data paths like `/etc/opencode/` or `C:\Program Files\`.

**Why Bad:**
- Breaks on non-standard installations
- Ignores XDG specification on Linux
- Fails for non-admin users

**Instead:** Use env-paths (Node) or directories (Rust) for platform-appropriate paths.

### Anti-Pattern 3: Monolithic Service Wrapper

**What:** Building a single binary that wraps Docker and runs as the service.

**Why Bad:**
- Adds unnecessary layer between service manager and Docker
- Complicates debugging
- Increases attack surface

**Instead:** Service definition should directly invoke Docker daemon. The CLI is for installation/management, not runtime.

### Anti-Pattern 4: Shared State Between CLIs

**What:** Assuming Node and Rust CLIs share runtime state.

**Why Bad:**
- They are separate processes
- Race conditions on config files
- No guaranteed ordering

**Instead:** Use file-based configuration with advisory locking. Each CLI reads config fresh.

---

## Build Order and Dependencies

### Dependency Graph

```
                    +------------------+
                    |  Docker Assets   |
                    |  (Dockerfile,    |
                    |   compose files) |
                    +--------+---------+
                             |
                             | (embedded/bundled at build time)
                             v
+----------------+   +-------+--------+   +----------------+
|  shared-types  |   |  Core Logic    |   |  Config Schema |
|  (TypeScript)  |   |  (per language)|   |  (JSON Schema) |
+-------+--------+   +-------+--------+   +-------+--------+
        |                    |                    |
        +--------------------+--------------------+
                             |
              +--------------+--------------+
              |                             |
      +-------v-------+             +-------v-------+
      |   cli-node    |             |   cli-rust    |
      |  (TypeScript) |             |    (Rust)     |
      +---------------+             +---------------+
              |                             |
              v                             v
      npm package                   cargo package
      (+ optional deps              (single binary
       per platform)                 per platform)
```

### Build Order

1. **Shared Types / Config Schema** (no dependencies)
   - JSON Schema for configuration validation
   - TypeScript type definitions (for Node CLI)

2. **Docker Assets** (no dependencies)
   - Dockerfile
   - docker-compose templates
   - These are static; "building" means validation/linting

3. **Core Logic Libraries** (depends on 1)
   - Node: Core TypeScript library with Docker/Service logic
   - Rust: Core library crate with same logic

4. **CLI Packages** (depends on 2, 3)
   - Node CLI: Wraps core library, adds CLI framework
   - Rust CLI: Wraps core library, adds CLI framework

5. **Distribution** (depends on 4)
   - npm: Build and publish npm package
   - cargo: Build release binaries for each platform

### Monorepo Tooling Recommendation

**Recommended:** pnpm workspaces + Turborepo for TypeScript, Cargo workspaces for Rust.

**Rationale:** Turborepo optimizes TypeScript builds with caching and parallelization but does not directly support Rust. Cargo workspaces handle Rust dependency management natively. The two can coexist in the same repository.

**turbo.json configuration:**
```json
{
  "$schema": "https://turborepo.dev/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    },
    "test": {
      "dependsOn": ["build"]
    },
    "lint": {}
  }
}
```

**Cargo.toml workspace:**
```toml
[workspace]
members = [
  "packages/cli-rust",
  "packages/opencode-core-rust"
]
```

### Build Scripts

```bash
# Full build (both languages)
pnpm install && pnpm build   # TypeScript
cargo build --release        # Rust

# CI/CD pipeline order
1. Lint (parallel): pnpm lint && cargo clippy
2. Test (parallel): pnpm test && cargo test
3. Build Node: pnpm build
4. Build Rust: cargo build --release (per target)
5. Package npm: npm pack
6. Package cargo: cargo-dist (generates platform binaries)
```

---

## Distribution Architecture

### npm Package Strategy

Following the [Sentry CLI pattern](https://sentry.engineering/blog/publishing-binaries-on-npm) for optional dependencies:

```
@opencode-cloud-service/cli          # Base package
@opencode-cloud-service/cli-darwin-x64
@opencode-cloud-service/cli-darwin-arm64
@opencode-cloud-service/cli-linux-x64
@opencode-cloud-service/cli-linux-arm64
@opencode-cloud-service/cli-win32-x64
```

Base package.json:
```json
{
  "name": "@opencode-cloud-service/cli",
  "bin": {
    "opencode-cloud-service": "./bin/cli.js"
  },
  "optionalDependencies": {
    "@opencode-cloud-service/cli-darwin-x64": "1.0.0",
    "@opencode-cloud-service/cli-darwin-arm64": "1.0.0",
    "@opencode-cloud-service/cli-linux-x64": "1.0.0"
  }
}
```

### Cargo Distribution

Use [cargo-dist](https://opensource.axo.dev/cargo-dist/book/installers/npm.html) for automated release builds:

```toml
# Cargo.toml
[package.metadata.dist]
installers = ["shell", "npm"]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc"
]
```

---

## Scalability Considerations

| Concern | Single User | Team (10 users) | Enterprise (100+ users) |
|---------|-------------|-----------------|-------------------------|
| Config storage | Local JSON file | Local JSON file | Consider centralized config |
| Service management | Single container | Single container | Container orchestration (K8s) |
| Logging | Local files | Local files | Centralized logging |
| Updates | Manual CLI | Manual CLI | Automated updates via service timer |

**Note:** This architecture is designed for single-node deployment. For enterprise/multi-node scenarios, consider Kubernetes operators instead of native service managers.

---

## Sources

### Official Documentation
- [Docker Engine SDK](https://docs.docker.com/reference/api/engine/sdk/) - Official Docker SDK documentation
- [Turborepo Repository Structure](https://turborepo.dev/docs/crafting-your-repository/structuring-a-repository) - Monorepo best practices

### Libraries (HIGH confidence)
- [Bollard - Rust Docker API](https://github.com/fussybeaver/bollard) - Async Docker client for Rust
- [Dockerode - Node.js Docker API](https://github.com/apocas/dockerode) - Docker Remote API for Node.js
- [env-paths](https://github.com/sindresorhus/env-paths) - Cross-platform config paths for Node.js
- [cross-xdg](https://crates.io/crates/cross-xdg) - XDG paths for Rust

### Architecture References (MEDIUM confidence)
- [Cloudflared Service Management](https://deepwiki.com/cloudflare/cloudflared/2.3-service-management-commands) - Cross-platform service installation pattern
- [Sentry CLI npm Distribution](https://sentry.engineering/blog/publishing-binaries-on-npm) - Binary distribution via npm
- [cargo-dist npm installer](https://opensource.axo.dev/cargo-dist/book/installers/npm.html) - Rust to npm distribution

### Background Research (MEDIUM confidence)
- [Monorepo Architecture Guide](https://feature-sliced.design/blog/frontend-monorepo-explained) - Monorepo best practices 2025
- [Nhost Turborepo Configuration](https://nhost.io/blog/how-we-configured-pnpm-and-turborepo-for-our-monorepo) - pnpm + Turborepo setup
