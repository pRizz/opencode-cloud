# Architecture Patterns

**Project:** opencode-cloud
**CLI:** `occ` / `opencode-cloud`
**Domain:** Cross-platform CLI for deploying opencode as a persistent cloud service
**Researched:** 2026-01-18
**Updated:** 2026-01-25 (layout, platform packages, Node passthrough, distribution)
**Confidence:** HIGH

## Executive Summary

opencode-cloud is a polyglot monorepo providing a cross-platform CLI to run opencode as a persistent, containerized service. The **Rust CLI is the single source of truth**. The Node CLI is a thin wrapper that spawns the Rust binary and passes all arguments through. Shared logic lives in a Rust core library with NAPI-RS bindings (used by the core npm package). Docker operations use **Bollard** only; there is no Dockerode. Configuration is JSONC at XDG-compliant paths. Prebuilt binaries are distributed via npm **optionalDependencies** (platform-specific packages `@opencode-cloud/cli-node-*`), so `npm install opencode-cloud` works without a Rust toolchain.

---

## Recommended Architecture

```
opencode-cloud/
|
|-- packages/
|   |-- core/                    # Rust library + NAPI-RS (npm: @opencode-cloud/core)
|   |   |-- src/                 # config, docker, host, platform, singleton
|   |   |-- Dockerfile           # embedded via include_str! in dockerfile.rs
|   |-- cli-rust/                # Rust CLI (cargo: opencode-cloud, bin: occ)
|   |   |-- src/commands/        # start, stop, config, user, mount, host, etc.
|   |-- cli-node/                # Node CLI wrapper (npm: opencode-cloud)
|   |   |-- src/index.ts         # spawns occ, stdio inherit, platform binary resolution
|   |-- cli-node-darwin-arm64/   # Platform package: macOS Apple Silicon binary
|   |-- cli-node-darwin-x64/     # Platform package: macOS Intel binary
|   |-- cli-node-linux-x64/      # Platform package: Linux x64 glibc
|   |-- cli-node-linux-arm64/    # Platform package: Linux ARM64 glibc
|   |-- cli-node-linux-x64-musl/ # Platform package: Linux x64 musl (Alpine)
|   |-- cli-node-linux-arm64-musl/ # Platform package: Linux ARM64 musl (Alpine)
|
|-- schemas/                     # config.schema.json, config.example.jsonc
|-- scripts/                     # set-all-versions, release, check-dockerfile-updates
|-- .github/workflows/           # ci, build-cli-binaries, publish-npm, docker-publish, version-bump
|-- justfile                     # build, test, fmt, lint, run, build-node-cli-mac, etc.
|-- pnpm-workspace.yaml          # pnpm workspaces (core, cli-node, cli-rust, cli-node-*)
|-- Cargo.toml                   # Rust workspace (core, cli-rust)
|-- package.json                 # Root (optional scripts, devDeps)
```

### High-Level Component Diagram

```
+-------------------+     +-------------------+
|   npm/npx CLI     |     |    cargo CLI      |
|   (cli-node)      |     |   (cli-rust/occ)  |
|   passthrough     |     |   source of truth |
+--------+----------+     +--------+----------+
         | spawn(occ)              |
         +-------------------------+
                      |
         +------------v------------+
         |   packages/core         |
         |   (Rust lib + NAPI-RS)  |
         |   Docker (Bollard),     |
         |   config, platform      |
         +------------+------------+
                      |
         +------------v------------+
         |   Docker daemon         |
         |   (socket / remote)     |
         +------------+------------+
                      |
         +------------v------------+
         |   opencode container    |
         |   (ghcr.io/.../sandbox) |
         +-------------------------+
```

---

## Component Boundaries

| Component | Responsibility | Technology | Communicates With |
|-----------|---------------|------------|-------------------|
| **cli-node** | npm/npx entry point, spawn Rust binary, resolve platform binary from optionalDependencies | TypeScript, Node spawn | Rust binary (occ), platform packages |
| **cli-rust** | CLI parsing, all business logic, Docker, config, service lifecycle | Rust, clap | core library, Docker daemon, filesystem |
| **core** | Config paths, schema, Docker client (Bollard), container lifecycle, platform (systemd/launchd), host/tunnel | Rust, NAPI-RS | Published as @opencode-cloud/core for npm |
| **Platform packages** | Ship prebuilt `occ` binary per platform; `main` exports `binaryPath` | JSON + index.js | Consumed by cli-node via optionalDependencies |
| **Docker** | Image build/pull, container create/start/stop, logs, exec | Bollard | Docker daemon (socket or remote via SSH tunnel) |
| **Config** | JSONC at XDG paths, validation, migration | Rust (config module) | ~/.config/opencode-cloud/, ~/.local/share/opencode-cloud/ |

### Component Detail

#### 1. CLI Entry Points

**Rust CLI (occ / opencode-cloud):** Single source of truth. All commands implemented in `packages/cli-rust`. Uses `opencode-cloud-core` library. Entry points: `src/bin/occ.rs`, `src/bin/opencode-cloud.rs`.

**Node CLI (opencode-cloud):** Thin wrapper. Resolves binary from (1) platform optionalDependency, or (2) `packages/cli-node/bin/occ` (dev fallback). Spawns binary with `stdio: 'inherit'`, passes `argv.slice(2)`. No parsing, no Docker logic.

**Commands (identical for both):**

```
occ start | stop | restart | status | logs
occ config show | get | set | reset | env ...
occ setup
occ user add | remove | list | passwd | enable | disable
occ mount add | remove | list
occ update
occ install | uninstall
occ cockpit
occ host add | remove | list | show | edit | test | default
occ --help | --version
```

#### 2. Docker Management

Implemented in `packages/core` (Rust). Uses **Bollard** only.

| Operation | Description | Location |
|-----------|-------------|----------|
| Connection | Verify Docker daemon, categorize errors | `docker/client.rs` |
| Image | Build (embedded Dockerfile), pull, tag | `docker/image.rs` |
| Container | Create, start, stop, remove, inspect | `docker/container.rs` |
| Logs | Stream container output | `docker/` + CLI |
| Exec | Run commands in container (e.g. user management) | `docker/exec.rs` |
| Health | `/health` check | `docker/health.rs` |

**Connection:** Unix socket (`/var/run/docker.sock`), or remote via SSH tunnel (`host/tunnel.rs`). No Dockerode; no Node Docker client.

#### 3. Service Installation

User-mode **systemd** (Linux) and **launchd** (macOS). No Windows. Implemented in `packages/core` (`platform/systemd.rs`, `platform/launchd.rs`).

| Platform | Manager | Config Location |
|----------|---------|-----------------|
| Linux | systemd (user) | `~/.config/systemd/user/` |
| macOS | launchd (user) | `~/Library/LaunchAgents/` |

`occ install` / `occ uninstall` register the service; the unit invokes `docker` (or `occ start` as appropriate). See cloudflared-style service management.

#### 4. Configuration Management

**Paths (XDG-style):**

| Platform | Config | Data |
|----------|--------|------|
| Linux | `~/.config/opencode-cloud/` | `~/.local/share/opencode-cloud/` |
| macOS | `~/.config/opencode-cloud/` | `~/.local/share/opencode-cloud/` |
| Windows | `%APPDATA%\opencode-cloud\` | `%LOCALAPPDATA%\opencode-cloud\` |

**Config file:** `config.json` (JSONC). Schema: `schemas/config.schema.json`. Example: `schemas/config.example.jsonc`. Validation and paths in `packages/core` config module; paths use `directories` (Rust) for XDG-style resolution.

**Other files:** `hosts.json` (remote hosts), `opencode-cloud.pid` (singleton lock), plus state under data dir.

#### 5. Platform Packages (Node prebuilt binaries)

Six npm packages, one per platform:

| Package | Platform |
|---------|----------|
| `@opencode-cloud/cli-node-darwin-arm64` | macOS Apple Silicon |
| `@opencode-cloud/cli-node-darwin-x64` | macOS Intel |
| `@opencode-cloud/cli-node-linux-x64` | Linux x64 glibc |
| `@opencode-cloud/cli-node-linux-arm64` | Linux ARM64 glibc |
| `@opencode-cloud/cli-node-linux-x64-musl` | Linux x64 musl (Alpine) |
| `@opencode-cloud/cli-node-linux-arm64-musl` | Linux ARM64 musl (Alpine) |

Each has `package.json` with `os`, `cpu`, `libc` (Linux), `files: ["bin"]`, `main: "index.js"`. `index.js` exports `binaryPath` to `bin/occ`. The main package `opencode-cloud` declares them as `optionalDependencies` (workspace:* in dev). npm installs only the matching platform package. `cli-node` resolves binary via `require(platformPackage).binaryPath` or falls back to `packages/cli-node/bin/occ` for development.

---

## Data Flow

### First-Run / Setup

```
User runs: npx opencode-cloud setup   (or first occ <cmd>)

1. Node: resolve binary (platform pkg or bin/occ)
2. Node: spawn(occ, ['setup', ...]), stdio inherit
3. Rust: clap parses, runs setup command
4. Rust: wizard prompts (auth, port, hostname, image source)
5. Core: ensure config dir, write config.json
6. Core: Docker check, optional image pull/build
7. Core: create container, users; start
8. Rust: print status, exit 0
```

### Start Flow

```
User runs: occ start [--host <name>]

1. Rust: load config, resolve Docker client (local or --host tunnel)
2. Core: check image, create container if needed (bind mounts, etc.)
3. Core: start container
4. Rust: update state, print URL / status
```

### Config Update Flow

```
User runs: occ config set key value

1. Rust: parse, load config, validate
2. Core: update config, write config.json
3. Rust: hint restart if needed
```

---

## Patterns to Follow

### Pattern 1: Rust Single Source of Truth, Node Passthrough

**What:** All CLI behavior lives in the Rust CLI. The Node CLI only finds and spawns the Rust binary.

**Why:** Guarantees perfect parity. No duplicate logic. Single place to add commands (Rust only).

**Implementation:**

- `packages/cli-node/src/index.ts`: `getPlatformPackage()` → `resolveBinaryPath()` → `spawn(binaryPath, argv.slice(2), { stdio: 'inherit' })`. No command parsing.
- New commands: add to `cli-rust` only; Node automatically passes them through.

### Pattern 2: Platform Abstraction in Core

**What:** Platform-specific service and path logic behind a common interface in `core`.

**Why:** Keeps `cli-rust` agnostic; core handles systemd vs launchd, XDG paths, etc.

**Implementation:** `platform/mod.rs`, `platform/systemd.rs`, `platform/launchd.rs`; `config/paths.rs` for XDG.

### Pattern 3: optionalDependencies for Prebuilt Binaries

**What:** Platform-specific npm packages with `os`/`cpu`/`libc`. Main package `optionalDependencies`; install pulls only the matching one.

**Why:** Zero-compile install for users. Same pattern as esbuild, swc, Sentry CLI.

**Implementation:** `packages/cli-node` optionalDeps → `@opencode-cloud/cli-node-*`. CI: `build-cli-binaries.yml` builds `occ` per target, `publish-npm` publishes platform packages then main.

### Pattern 4: Justfile for Orchestration

**What:** `just` for build, test, fmt, lint, run, release, publish.

**Why:** Single entrypoint, cross-platform, no Turborepo. Matches Rust-focused workflow.

**Implementation:** `just build`, `just test`, `just run -- args`, `just build-node-cli-mac`, etc.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Direct Docker CLI Shelling

**What:** Invoking `docker` via subprocess instead of using the Docker API.

**Why Bad:** Fragile output parsing, requires Docker CLI, poor error handling.

**Instead:** Use Bollard (Rust) to talk to the Docker daemon.

### Anti-Pattern 2: Hardcoded Paths

**What:** Fixed paths like `/etc/opencode/` or `C:\Program Files\`.

**Why Bad:** Breaks XDG, non-standard installs, non-admin users.

**Instead:** `config/paths.rs` using the `directories` crate (XDG-style).

### Anti-Pattern 3: Duplicating CLI Logic in Node

**What:** Implementing commands or Docker logic in the Node wrapper.

**Why Bad:** Divergence from Rust, two sources of truth.

**Instead:** Node only spawns `occ`; all logic in Rust.

### Anti-Pattern 4: Shared Runtime State Between Node and Rust

**What:** Assuming Node and Rust CLIs share process state.

**Why Bad:** Separate processes; races on config.

**Instead:** File-based config, advisory locking; each process reads afresh.

---

## Build Order and Dependencies

### Dependency Graph

```
                    +------------------+
                    |  Core (Rust)     |
                    |  Docker, config, |
                    |  platform, host  |
                    +--------+---------+
                             |
         +-------------------+-------------------+
         |                                       |
+--------v--------+                     +--------v--------+
|  cli-rust       |                     |  cli-node       |
|  (Rust CLI)     |                     |  (Node wrapper) |
|  occ binary     |                     |  + optionalDeps |
+-----------------+                     |  cli-node-*     |
         |                              +-----------------+
         |                                       |
         v                                       v
  cargo install                          npm install
  (or build-cli-binaries                 (platform binary
   for npm publish)                       per os/cpu/libc)
```

### Build Order

1. **Core** – `cargo build -p opencode-cloud-core`; `pnpm -C packages/core build` (NAPI-RS).
2. **cli-rust** – `cargo build -p opencode-cloud`; produces `occ` / `opencode-cloud` binaries.
3. **cli-node** – `pnpm -C packages/cli-node build` (tsc). Depends on optionalDependencies for runtime; dev uses `bin/occ`.
4. **Platform packages** – No build step; CI copies `occ` into each `packages/cli-node-*/bin/`.

### Tooling

- **Rust:** Cargo workspace (`core`, `cli-rust`). `rust-toolchain.toml` pins version.
- **Node:** pnpm workspaces. Workspace: `core`, `cli-node`, `cli-rust`, `cli-node-darwin-arm64`, etc.
- **Orchestration:** `justfile` (build, test, fmt, lint, run, build-node-cli-mac, run-node-cli-mac, publish-*).
- **CI:** `ci.yml`, `build-cli-binaries.yml`, `publish-npm.yml`, `docker-publish.yml`, `version-bump.yml`.

---

## Distribution Architecture

### npm

- **Main package:** `opencode-cloud`. Bin: `opencode-cloud`, `occ` → `dist/index.js`.
- **optionalDependencies:** All six `@opencode-cloud/cli-node-*` packages (workspace:* in repo; concrete versions on publish).
- **Install:** `npm install opencode-cloud` pulls the matching platform package; Node CLI resolves `binaryPath` and spawns `occ`. No Rust required.
- **Publish:** `publish-npm` workflow runs `build-cli-binaries` (matrix build), downloads artifacts, publishes platform packages, then `@opencode-cloud/core`, then `opencode-cloud`.

### Crates.io

- **opencode-cloud-core** – Core library (used by cli-rust; also npm via NAPI-RS).
- **opencode-cloud** – CLI crate, binaries `occ` and `opencode-cloud`.

### Docker

- Image: `prizz/opencode-cloud-sandbox` (Docker Hub primary, with GHCR mirror fallback). Built via `docker-publish` workflow. Dockerfile lives in `packages/core` and is embedded in the Rust build.

---

## Scalability Considerations

| Concern | Single User | Team | Enterprise |
|---------|-------------|------|------------|
| Config | Local JSONC | Local JSONC | Centralized config (future) |
| Service | Single container | Single container | Orchestration (e.g. K8s) |
| Logs | Local / Docker | Local / Docker | Centralized logging |
| Updates | `occ update` | `occ update` | Automated (e.g. timer) |
| Hosts | `occ host` (SSH tunnel) | Multiple hosts | Scale via host management |

The design targets single-node deployment. Multi-node or enterprise use cases may need orchestration or operators rather than only systemd/launchd.

---

## Sources

### Official / Ecosystem

- [Bollard](https://github.com/fussybeaver/bollard) – Async Docker client for Rust
- [NAPI-RS](https://napi.rs/) – Rust ↔ Node bindings
- [Docker Engine API](https://docs.docker.com/reference/api/engine/sdk/)
- [just](https://github.com/casey/just) – Command runner

### Architecture References

- [Cloudflared service management](https://deepwiki.com/cloudflare/cloudflared/2.3-service-management-commands) – systemd/launchd pattern
- [Sentry CLI npm distribution](https://sentry.engineering/blog/publishing-binaries-on-npm) – optionalDependencies binaries
- [esbuild / swc platform packages](https://esbuild.github.io/getting-started/#install-esbuild) – os/cpu platform splitting

### Project Docs

- `CLAUDE.md` – Pre-commit, commands, structure
- `CONTRIBUTING.md` – CLI architecture, adding commands
- `packages/cli-rust/README.md` – Rust CLI as source of truth
