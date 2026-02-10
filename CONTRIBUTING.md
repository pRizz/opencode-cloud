# Contributing to opencode-cloud

Thank you for your interest in contributing to opencode-cloud! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- **Rust 1.85+** (for Rust 2024 edition)
- **Node.js 20+**
- **Bun 1.3.8+**
- **just** (task runner)

### Installation

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just
cargo install just
# or: brew install just

# Install Bun
curl -fsSL https://bun.sh/install | bash

# Clone the repository
# GitHub (primary)
git clone https://github.com/pRizz/opencode-cloud.git

# Gitea (mirror)
git clone https://gitea.com/pRizz/opencode-cloud.git
cd opencode-cloud

# Initialize submodule checkout
git submodule update --init --recursive packages/opencode

# Bun is required for this repo
bun --version

# One-time setup (hooks + deps + submodule bootstrap)
just setup

# Build everything
just build

# Recommended local dev runtime (local submodule + cached sandbox rebuild)
just dev
```

### Running Tests

```bash
# Run all tests
just test

# Run only Rust tests
just test-rust

# Run only Node tests
just test-node
```

### Linting and Formatting

```bash
# Check linting
just lint

# Auto-format code
just fmt
```

### README Badge Sync

The root README (`README.md`) and submodule README (`packages/opencode/README.md`) have distinct generated badge blocks.
Both are sourced from `packages/opencode/packages/fork-ui/src/readme-badge-catalog.ts`.

```bash
# Regenerate both README badge sections
just sync-readme-badges

# Validate badge sections are in sync (also run by just lint)
just check-readme-badges
```

Do not hand-edit badge lines between generated marker comments in either README.

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

### Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that don't affect meaning (formatting, etc.)
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or correcting tests
- `chore`: Changes to build process or auxiliary tools

### Examples

```
feat(cli): add --json flag for machine-readable output
fix(config): handle missing config directory on first run
docs(readme): add installation instructions for Windows
```

## Pull Request Process

1. **Fork** the repository and create your branch from `main`
2. **Make your changes** following our coding standards
3. **Write tests** for any new functionality
4. **Ensure all tests pass**: `just test`
5. **Ensure linting passes**: `just lint`
6. **Update documentation** if needed
7. **Submit a pull request** with a clear description

## Project Structure

```
opencode-cloud/
├── packages/
│   ├── core/           # Rust core library + NAPI bindings
│   ├── cli-rust/       # Rust CLI binary (source of truth)
│   └── cli-node/       # Node.js CLI wrapper (passthrough)
├── Cargo.toml          # Rust workspace root
├── package.json        # Node.js workspace root
├── bun.lock            # Bun lockfile
└── justfile            # Task orchestration
```

## CLI Architecture

opencode-cloud has two CLI entry points that work together:

### Two Entry Points

1. **Rust CLI** (`packages/cli-rust`) - **Source of truth**
   - Standalone binary: `occ`
   - Contains all command logic
   - Can be installed via `cargo install opencode-cloud`

2. **Node CLI** (`packages/cli-node`) - **Transparent passthrough**
   - Wrapper that spawns the Rust binary
   - Published to npm as `opencode-cloud`
   - Zero logic - just `spawn(rustBinary, args, { stdio: 'inherit' })`

### How It Works

When a user runs `npx opencode-cloud start`:

1. Node CLI (`packages/cli-node/src/index.ts`) receives the command
2. It spawns the Rust binary with all arguments passed through
3. Rust CLI (`packages/cli-rust`) handles the command
4. Output flows back through unchanged

This means:
- **TTY detection works** - Colors and interactive prompts preserve their behavior
- **Exit codes propagate** - Scripts can rely on proper exit codes
- **No duplication** - Command logic lives in one place
- **Node changes rarely needed** - Adding commands only requires Rust updates

### Adding New Commands

To add a new command (e.g., `occ shell`):

#### 1. Define the command struct

Create `packages/cli-rust/src/commands/shell.rs`:

```rust
use clap::Args;
use anyhow::Result;

#[derive(Args)]
pub struct ShellArgs {
    /// Shell to use (default: bash)
    #[arg(short, long, default_value = "bash")]
    shell: String,
}

pub async fn cmd_shell(args: &ShellArgs, quiet: bool) -> Result<()> {
    // Implementation here
    todo!()
}
```

#### 2. Register in commands/mod.rs

Add to `packages/cli-rust/src/commands/mod.rs`:

```rust
mod shell;
pub use shell::{ShellArgs, cmd_shell};
```

#### 3. Add to CLI enum

Update `packages/cli-rust/src/lib.rs`:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands

    /// Open a shell in the container
    Shell(commands::ShellArgs),
}
```

#### 4. Add command handler

In the `match cli.command` block in `lib.rs`:

```rust
Some(Commands::Shell(args)) => {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(commands::cmd_shell(&args, cli.quiet))
}
```

#### 5. Build and test

```bash
# Build both CLIs
just build

# Test Rust CLI directly
./target/release/occ shell --help

# Test Node CLI (uses Rust binary)
./packages/cli-node/bin/occ shell --help
```

**That's it!** No changes needed in `packages/cli-node` - it automatically passes through the new command.

### Testing CLI Commands

```bash
# Test specific command
cargo test -p cli-rust cmd_shell

# Integration test
./target/release/occ shell --shell zsh

# Verify Node wrapper works
node packages/cli-node/dist/index.js shell --shell zsh
```

### Example: Adding a "shell" Command

Let's walk through a complete example of adding `occ shell` to access the container terminal.

**Step 1: Create the command module**

```bash
touch packages/cli-rust/src/commands/shell.rs
```

**Step 2: Implement the command**

```rust
use anyhow::Result;
use clap::Args;
use opencode_cloud_core::DockerClient;

#[derive(Args)]
pub struct ShellArgs {
    /// Shell to use (default: bash)
    #[arg(short, long, default_value = "bash")]
    shell: String,

    /// User to run shell as
    #[arg(short, long)]
    user: Option<String>,
}

pub async fn cmd_shell(args: &ShellArgs, quiet: bool) -> Result<()> {
    let client = DockerClient::new()?;

    if !client.is_container_running().await? {
        anyhow::bail!("Container is not running. Start it with: occ start");
    }

    // Exec into container with the specified shell
    client.exec_interactive(&args.shell, args.user.as_deref()).await?;

    Ok(())
}
```

**Step 3-5: Register and build** (as shown above)

Now users can run:
```bash
occ shell                    # Default bash
occ shell --shell zsh        # Custom shell
npx opencode-cloud shell     # Works via Node wrapper too!
```

## Pre-Commit Checklist

Before submitting a PR, ensure:

- [ ] `just fmt` - Code is formatted
- [ ] `just lint` - No linting errors
- [ ] `just test` - All tests pass
- [ ] `just build` - Release build succeeds
- [ ] New commands documented in code with `///` doc comments
- [ ] Breaking changes noted in PR description

## Code Style

For detailed code style guidelines, see [CLAUDE.md](./CLAUDE.md).

### Rust Quick Reference

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Prefer `?` for error propagation over `unwrap()`
- Document public APIs with `///` comments

### TypeScript Quick Reference

- Use strict mode
- Follow ESM conventions
- Keep the Node CLI thin - it should only spawn the Rust binary
- Logic belongs in Rust core, not Node wrapper

## Questions?

Feel free to open an issue for any questions about contributing.
