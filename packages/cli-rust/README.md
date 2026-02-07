# opencode-cloud Rust CLI

**This is the source of truth for all CLI commands.**

The Rust CLI (`occ`) contains the complete implementation of all opencode-cloud commands. The Node CLI (`packages/cli-node`) is a transparent passthrough wrapper that spawns this binary.

## Architecture

opencode-cloud uses a dual-CLI strategy:

1. **Rust CLI** (this package) - Complete implementation, all logic lives here
2. **Node CLI** (`packages/cli-node`) - Wrapper that spawns Rust binary with `stdio: 'inherit'`

This architecture provides:
- **Single source of truth** - All command logic in one place (Rust)
- **No duplication** - Node wrapper has zero command logic
- **Automatic sync** - New Rust commands instantly work via Node
- **Full TTY support** - Colors, interactive prompts, progress bars all work
- **Cross-platform** - Users choose their preferred installation method

## Building

### Development Build

```bash
# Build Rust CLI only
cargo build -p opencode-cloud

# Run directly
./target/debug/occ --help

# Or via cargo
cargo run -p opencode-cloud -- start
```

### Release Build

```bash
cargo build -p opencode-cloud --release
./target/release/occ --help
```

### Full Project Build

```bash
# Build all packages (core + CLIs)
just build
```

## Project Structure

```
packages/cli-rust/
├── src/
│   ├── bin/
│   │   ├── occ.rs              # Binary entry point (occ)
│   │   └── opencode-cloud.rs   # Binary entry point (opencode-cloud)
│   ├── commands/
│   │   ├── mod.rs              # Command registry
│   │   ├── start.rs            # occ start
│   │   ├── stop.rs             # occ stop
│   │   ├── status.rs           # occ status
│   │   ├── config/             # occ config subcommands
│   │   ├── user/               # occ user subcommands
│   │   ├── mount/              # occ mount subcommands
│   │   └── ...                 # Other commands
│   ├── output/
│   │   ├── spinner.rs          # Progress spinners
│   │   ├── colors.rs           # Terminal color utilities
│   │   ├── errors.rs           # Error formatting
│   │   └── urls.rs             # URL formatting helpers
│   ├── wizard/
│   │   ├── mod.rs              # Interactive setup wizard
│   │   ├── network.rs          # Network prompts
│   │   ├── summary.rs          # Wizard summary output
│   │   └── prechecks.rs        # Environment prechecks
│   └── lib.rs                  # Shared CLI implementation
├── Cargo.toml
└── README.md                   # This file
```

## Adding Commands

To add a new command (e.g., `occ shell`):

### 1. Create the command module

```bash
touch src/commands/shell.rs
```

### 2. Implement the command

```rust
use anyhow::Result;
use clap::Args;
use opencode_cloud_core::DockerClient;

#[derive(Args)]
pub struct ShellArgs {
    /// Shell to use (default: bash)
    #[arg(short, long, default_value = "bash")]
    shell: String,
}

pub async fn cmd_shell(args: &ShellArgs, quiet: bool) -> Result<()> {
    let client = DockerClient::new()?;

    if !client.is_container_running().await? {
        anyhow::bail!("Container is not running. Start it with: occ start");
    }

    client.exec_interactive(&args.shell, None).await?;
    Ok(())
}
```

### 3. Register in `commands/mod.rs`

```rust
mod shell;
pub use shell::{ShellArgs, cmd_shell};
```

### 4. Add to CLI enum in `src/lib.rs`

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    /// Open a shell in the container
    Shell(commands::ShellArgs),
}
```

### 5. Add command handler in `src/lib.rs`

In the `match cli.command` block:

```rust
Some(Commands::Shell(args)) => {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(commands::cmd_shell(&args, cli.quiet))
}
```

### 6. Test

```bash
# Build
cargo build -p opencode-cloud

# Test Rust CLI
./target/debug/occ shell --help
./target/debug/occ shell

# Node CLI automatically works too
node packages/cli-node/dist/index.js shell
```

**That's all!** No changes needed in `packages/cli-node` - it automatically delegates to the new Rust command.

## Testing

```bash
# Run all tests
cargo test -p opencode-cloud

# Run specific test
cargo test -p opencode-cloud cmd_shell

# With verbose output
cargo test -p opencode-cloud -- --nocapture
```

## Installation

### From Source (Development)

```bash
cargo install --path packages/cli-rust
```

This installs the Rust CLI directly from a local checkout.

### From crates.io (Users)

```bash
cargo install opencode-cloud
```

### Via npm (Users)

```bash
npm install -g opencode-cloud
# or
npx opencode-cloud
```

The npm package includes this Rust CLI and compiles it during installation.

## Code Style

See [CLAUDE.md](../../CLAUDE.md) for detailed code style guidelines.

Quick reference:
- Prefer `?` for error propagation over `unwrap()`
- Use `let...else` for early returns
- Prefix `Option` types with `maybe_`
- Document public APIs with `///` comments
- Run `cargo fmt` and `cargo clippy` before committing

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for full contribution guidelines.
