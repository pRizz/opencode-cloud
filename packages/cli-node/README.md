# opencode-cloud

[![CI](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml)
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

# View configuration
occ config show

# More commands coming in future releases:
# occ start    - Start the service
# occ stop     - Stop the service
# occ status   - Check service status
# occ logs     - View service logs
```

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
