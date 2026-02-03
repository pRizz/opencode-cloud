# opencode-cloud

[![CI](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml)
[![Mirror](https://img.shields.io/badge/mirror-gitea-blue?logo=gitea)](https://gitea.com/pRizz/opencode-cloud)
[![crates.io](https://img.shields.io/crates/v/opencode-cloud.svg)](https://crates.io/crates/opencode-cloud)
[![GHCR](https://img.shields.io/badge/ghcr.io-sandbox-blue?logo=github)](https://github.com/pRizz/opencode-cloud/pkgs/container/opencode-cloud-sandbox)
[![Docker Hub](https://img.shields.io/docker/v/prizz/opencode-cloud-sandbox?label=docker&sort=semver)](https://hub.docker.com/r/prizz/opencode-cloud-sandbox)
[![docs.rs](https://docs.rs/opencode-cloud/badge.svg)](https://docs.rs/opencode-cloud)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> [!WARNING]
> This tool is still a work in progress and is rapidly evolving. Expect frequent updates and breaking changes. Follow updates at https://github.com/pRizz/opencode-cloud (mirror: https://gitea.com/pRizz/opencode-cloud). Stability will be announced at some point. Use with caution.

A production-ready toolkit for deploying and managing [opencode](https://github.com/anomalyco/opencode) as a persistent cloud service, **sandboxed inside a Docker container** for isolation and security.

This project uses the opencode fork at https://github.com/pRizz/opencode, which adds additional authentication and security features.

## Quick install (cargo)

```bash
cargo install opencode-cloud
opencode-cloud --version
```

## Quick install (npm)

```bash
npx opencode-cloud@latest --version
```

```bash
bunx opencode-cloud@latest --version
```

Or install globally:
```bash
npm install -g opencode-cloud
opencode-cloud --version
```

## Deploy to AWS

[![Deploy to AWS](https://s3.amazonaws.com/cloudformation-examples/cloudformation-launch-stack.png)](https://console.aws.amazon.com/cloudformation/home#/stacks/create/review?templateURL=https://opencode-cloud-templates.s3.us-east-2.amazonaws.com/cloudformation/opencode-cloud-quick.yaml)

Quick deploy provisions a private EC2 instance behind a public ALB with HTTPS.
**A domain name is required** for ACM certificate validation.
**A Route53 hosted zone ID is required** for automated DNS validation.

Docs: `docs/deploy/aws.md` (includes teardown steps and S3 hosting setup for forks)
Credentials: `docs/deploy/aws.md#retrieving-credentials`

## Features

- **Sandboxed execution** - opencode runs inside a Docker container, isolated from your host system
- **Persistent environment** - Your projects, settings, and shell history persist across restarts
- **Cross-platform CLI** (`opencode-cloud` / `occ`) - Works on Linux and macOS
- **Service lifecycle commands** - start, stop, restart, status, logs
- **Platform service integration** - systemd (Linux) / launchd (macOS) for auto-start on boot
- **Remote host management** - Manage opencode containers on remote servers via SSH

## How it works

opencode-cloud runs opencode inside a Docker container, providing:

- **Isolation** - opencode and its AI-generated code run in a sandbox, separate from your host system
- **Reproducibility** - The container includes a full development environment (languages, tools, runtimes)
- **Persistence** - Docker volumes preserve your work across container restarts and updates
- **Security** - Network exposure is opt-in; by default, the service only binds to localhost

The CLI manages the container lifecycle, so you don't need to interact with Docker directly.

## Docker Images

The sandbox container image is named **`opencode-cloud-sandbox`** (not `opencode-cloud`) to clearly distinguish it from the CLI tool. The preferred way to use and manage the image is via the opencode-cloud CLI ([GitHub](https://github.com/pRizz/opencode-cloud), mirror: https://gitea.com/pRizz/opencode-cloud). It handles image pulling, container setup, and upgrades for you.

**Why use the CLI?** It configures volumes, ports, and upgrades safely, so you don’t have to manage `docker run` flags or image updates yourself.

The image is published to both registries:

| Registry | Image |
|----------|-------|
| GitHub Container Registry | [`ghcr.io/prizz/opencode-cloud-sandbox`](https://github.com/pRizz/opencode-cloud/pkgs/container/opencode-cloud-sandbox) |
| Docker Hub | [`prizz/opencode-cloud-sandbox`](https://hub.docker.com/r/prizz/opencode-cloud-sandbox) |

Pull commands:

Docker Hub:
```bash
docker pull prizz/opencode-cloud-sandbox:latest
```

GitHub Container Registry:
```bash
docker pull ghcr.io/prizz/opencode-cloud-sandbox:latest
```

**For most users:** Just use the CLI - it handles image pulling/building automatically:
```bash
occ start  # Pulls or builds the image as needed
```

## Requirements

- **Rust 1.85+** - Install via [rustup](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Docker** - For running the opencode container
- **Supported platforms** - Linux and macOS

## Installation

### Via cargo (recommended)

```bash
cargo install opencode-cloud
occ --version
```

### Via npm

```bash
npx opencode-cloud@latest --version
```

```bash
bunx opencode-cloud@latest --version
```

Or install globally:
```bash
npm install -g opencode-cloud
occ --version
```

## First run

```bash
# Install as a system service (recommended for background use)
occ install

# Start the service
occ start
```

### From source (install locally)

```bash
# GitHub (primary)
git clone https://github.com/pRizz/opencode-cloud.git

# Gitea (mirror)
git clone https://gitea.com/pRizz/opencode-cloud.git
cd opencode-cloud
cargo install --path packages/cli-rust
```

### From source (development run)

```bash
# GitHub (primary)
git clone https://github.com/pRizz/opencode-cloud.git

# Gitea (mirror)
git clone https://gitea.com/pRizz/opencode-cloud.git
cd opencode-cloud
just build
cargo run -p opencode-cloud -- --version
```

## Usage

```bash
# Show version
occ --version

# Start the service (builds Docker container on first run, ~10-15 min)
occ start

# Start on a custom port
occ start --port 8080

# Start and open browser
occ start --open

# Check service status
occ status

# View logs
occ logs

# Follow logs in real-time
occ logs -f

# Stop the service
occ stop

# Restart the service
occ restart

# Update the opencode-cloud CLI binary
occ update cli

# Update the opencode-cloud container image
occ update container

# Update opencode inside the container
occ update opencode

# Update opencode using a specific branch or commit
occ update opencode --branch dev
occ update opencode --commit <sha>

# Remove the container (keeps volumes)
occ reset container

# Remove container and volumes (data loss)
occ reset container --volumes --force

# Clean bind mount contents (data loss)
occ mount clean --force

# Purge bind mounts (data loss, removes config entries)
occ mount clean --purge --force

# Factory reset host (container, volumes, mounts, config/data)
occ reset host --force

### Webapp-triggered update (command file)

When running in foreground mode (for example via `occ install`, which uses `occ start --no-daemon`),
the host listens for a command file on a bind mount. The webapp can write a simple JSON payload
to request an update.

Default paths (with default bind mounts enabled):
- Host: `~/.local/state/opencode/opencode-cloud/commands/update-command.json`
- Container: `/home/opencode/.local/state/opencode/opencode-cloud/commands/update-command.json`

Example payload:
```json
{
  "command": "update_opencode",
  "request_id": "webapp-1234",
  "branch": "dev"
}
```

The host writes the result to:
`~/.local/state/opencode/opencode-cloud/commands/update-command.result.json`

# Install as a system service (starts on login/boot)
occ install

# Uninstall the system service
occ uninstall

# View configuration
occ config show
```

## Authentication

opencode-cloud uses **PAM (Pluggable Authentication Modules)** for authentication. Users created via `occ user add` authenticate to the opencode web UI.

### Creating Users

Create a user with a password:
```bash
occ user add <username>
```

Generate a random password:
```bash
occ user add <username> --generate
```

### Managing Users

- List users: `occ user list`
- Change password: `occ user passwd <username>`
- Remove user: `occ user remove <username>`
- Enable/disable account: `occ user enable <username>` / `occ user disable <username>`

### User Persistence

User accounts (including password hashes and lock status) persist across container updates and rebuilds.
The CLI stores user records in a managed Docker volume mounted at `/var/lib/opencode-users` inside the container.
No plaintext passwords are stored on the host.

### Legacy Authentication Fields

The `auth_username` and `auth_password` config fields are **deprecated** and ignored. They are kept in the config schema for backward compatibility with existing deployments, but new users should be created via `occ user add` instead.

To migrate from legacy fields:
1. Create a PAM user: `occ user add <username>`
2. The legacy fields will be automatically cleared on next config save

### Rebuilding the Docker Image

When developing locally or after updating opencode-cloud, you may need to rebuild the Docker image to pick up changes in the embedded Dockerfile:

```bash
# Rebuild using Docker cache (fast - only rebuilds changed layers)
occ start --cached-rebuild

# Rebuild from scratch without cache (slow - for troubleshooting)
occ start --full-rebuild
```

**`--cached-rebuild`** (recommended for most cases):
- Uses Docker layer cache for fast rebuilds
- Only rebuilds layers that changed (e.g., if only the CMD changed, it's nearly instant)
- Stops and removes any existing container before rebuilding

**`--full-rebuild`** (for troubleshooting):
- Ignores Docker cache and rebuilds everything from scratch
- Takes 10-15 minutes but guarantees a completely fresh image
- Use when cached rebuild doesn't fix issues

**When to rebuild:**
- After pulling updates to opencode-cloud → use `--cached-rebuild`
- When modifying the Dockerfile during development → use `--cached-rebuild`
- When the container fails to start due to image issues → try `--cached-rebuild` first, then `--full-rebuild`
- When you want a completely fresh environment → use `--full-rebuild`

## Configuration

Configuration is stored at:
- Linux/macOS: `~/.config/opencode-cloud/config.json`

Data (PID files, etc.) is stored at:
- Linux/macOS: `~/.local/share/opencode-cloud/`

## Development

```bash
# Install dependencies
pnpm install

# Configure git hooks (once after cloning)
git config core.hooksPath .githooks

# Build everything
just build

# Compile and run occ (arguments automatically get passed to the binary)
just run --version

# Run tests
just test

# Format and lint
just fmt
just lint
```

> **Note:** The git hooks automatically sync `README.md` to npm package directories on commit.

## Architecture

This is a monorepo with:
- `packages/core` - Rust core library
- `packages/cli-rust` - Rust CLI binary (recommended)
- `packages/cli-node` - Node.js CLI (fully supported and in parity with the Rust CLI)

### Cargo.toml Sync Requirement

The `packages/core/Cargo.toml` file must use **explicit values** rather than `workspace = true` references.

When updating package metadata (version, edition, rust-version, etc.), keep both files in sync:
- `Cargo.toml` (workspace root)
- `packages/core/Cargo.toml`

Use `scripts/set-all-versions.sh <version>` to update versions across all files automatically.

## License

MIT
