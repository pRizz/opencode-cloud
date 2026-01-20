# opencode-cloud

[![CI](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/opencode-cloud.svg)](https://crates.io/crates/opencode-cloud)
[![npm](https://img.shields.io/npm/v/opencode-cloud.svg)](https://www.npmjs.com/package/opencode-cloud)
[![docs.rs](https://docs.rs/opencode-cloud/badge.svg)](https://docs.rs/opencode-cloud)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A production-ready toolkit for deploying [opencode](https://github.com/anomalyco/opencode) as a persistent cloud service.

## Features

- Cross-platform CLI (`opencode-cloud` / `occ`)
- Docker container management
- Service lifecycle commands (start, stop, status, logs)
- Platform service integration (systemd/launchd)
- XDG-compliant configuration
- Singleton enforcement (one instance per host)

## Requirements

### For npm installation

- **Node.js 20+**
- **Rust 1.82+** (for compiling native bindings)
  - Install via [rustup](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### For cargo installation

- **Rust 1.82+**

## Installation

### Via npm (compiles from source)

```bash
npx opencode-cloud --version
```

Or install globally:

```bash
npm install -g opencode-cloud
occ --version
```

### Via cargo

```bash
cargo install opencode-cloud
opencode-cloud --version
```

### From source

```bash
git clone https://github.com/pRizz/opencode-cloud.git
cd opencode-cloud
just build
cargo run -p opencode-cloud -- --version
```

## Usage

```bash
# Show version
occ --version

# Start the service (builds image on first run)
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

# Install as a system service (starts on login/boot)
occ install

# Uninstall the system service
occ uninstall

# View configuration
occ config show
```

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
- `packages/core` - Rust core library with NAPI-RS bindings
- `packages/cli-rust` - Rust CLI binary
- `packages/cli-node` - Node.js CLI wrapper (calls into core via NAPI)

The npm package compiles the Rust core on install (no prebuilt binaries).

## License

MIT
