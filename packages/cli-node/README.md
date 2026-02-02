# opencode-cloud Node.js CLI

Cross-platform CLI for opencode-cloud with prebuilt binaries for major platforms.

## Installation

```bash
npx opencode-cloud@latest --version
```

```bash
bunx opencode-cloud@latest --version
```

```bash
npm install -g opencode-cloud
```

No Rust toolchain required — npm automatically downloads the correct binary for your platform.

## Supported Platforms

| Platform | Architecture | Package |
|----------|--------------|---------|
| macOS | Apple Silicon (arm64) | @opencode-cloud/cli-node-darwin-arm64 |
| macOS | Intel (x64) | @opencode-cloud/cli-node-darwin-x64 |
| Linux | x64 (glibc) | @opencode-cloud/cli-node-linux-x64 |
| Linux | ARM64 (glibc) | @opencode-cloud/cli-node-linux-arm64 |
| Linux | x64 (musl/Alpine) | @opencode-cloud/cli-node-linux-x64-musl |
| Linux | ARM64 (musl/Alpine) | @opencode-cloud/cli-node-linux-arm64-musl |

Windows support planned for a future release.

## How it works

1. When you install `opencode-cloud`, npm downloads a platform-specific package based on your OS and architecture
2. The main package finds the binary from the platform package
3. All CLI invocations spawn the Rust binary with transparent passthrough (stdio: inherit)
4. Exit codes, colors, and TTY detection are preserved

## Development

When developing locally, place the Rust binary in the Node package `bin/` directory:

```bash
cargo build --release
cp target/release/occ packages/cli-node/bin/
```

Then run via `just run` or `node packages/cli-node/dist/index.js <args>`. The wrapper resolves the binary from `bin/` when platform packages are not used.

## Usage

```bash
occ start
occ status
occ stop
```

All commands are identical between the npm and cargo installations.

## License

MIT
