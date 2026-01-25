# opencode-cloud Node.js CLI

A lightweight Node.js wrapper for the opencode-cloud CLI. This package delegates all operations to the Rust binary for optimal performance and feature parity.

## How It Works

The Node.js CLI is a transparent passthrough wrapper:

1. **Spawns the Rust binary** (`occ`) with all provided arguments
2. **Inherits stdio** - Colors, TTY detection, and interactive prompts work seamlessly
3. **Propagates exit codes** - Exit codes from the Rust binary are passed through exactly
4. **Zero parsing overhead** - No command interception or parsing in JavaScript

This architecture ensures that the npm package always has 100% feature parity with the Rust CLI.

## Binary Placement

### Development

When developing locally, copy the built Rust binary to the Node package:

```bash
# Build the Rust CLI
cargo build --release

# Copy to Node package bin directory
cp target/release/occ packages/cli-node/bin/
```

### CI/CD

GitHub Actions workflows copy built binaries to `packages/cli-node/bin/` before running integration tests.

### End Users

**Current (Phase 18):** Users need to install the Rust CLI separately:

```bash
cargo install opencode-cloud
```

**Future (Phase 22):** Prebuilt binaries will be distributed via npm using `optionalDependencies`. The wrapper will automatically select the correct platform binary at install time.

## Installation

### Via npm (requires Rust)

```bash
npm install -g opencode-cloud
```

**Note:** Currently requires `cargo install opencode-cloud` to be run separately to provide the binary.

### Via Cargo (recommended)

```bash
cargo install opencode-cloud
```

This installs the native Rust binary directly without any Node.js wrapper.

## Usage

Both installation methods provide the same CLI:

```bash
# Start opencode instance
occ start

# Check status
occ status

# Stop instance
occ stop
```

All commands are identical between the npm and cargo installations.

## Architecture

```
┌─────────────────────────────────────┐
│  packages/cli-node/dist/index.js    │
│  (Node.js wrapper)                  │
└─────────────┬───────────────────────┘
              │ spawn()
              │ stdio: 'inherit'
              │
              ▼
┌─────────────────────────────────────┐
│  packages/cli-node/bin/occ          │
│  (Rust binary)                      │
│                                     │
│  All CLI logic lives here           │
└─────────────────────────────────────┘
```

## Future: Prebuilt Binaries (Phase 22)

Phase 22 will add automatic platform binary distribution:

- Binaries built for `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`
- Published as separate npm packages (e.g., `@opencode-cloud/darwin-arm64`)
- Main package uses `optionalDependencies` to download correct binary
- Install works without Rust toolchain

## License

MIT
