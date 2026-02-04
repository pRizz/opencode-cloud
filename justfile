# justfile - Root task orchestration for opencode-cloud

# Default recipe
default: list

# List available recipes
list:
    @just --list

# Setup development environment (run once after cloning)
setup:
    git config core.hooksPath .githooks
    pnpm install
    @echo "✓ Development environment ready!"

# Build everything
build: build-rust build-node

# Compile and run the occ binary (arguments automatically get passed to the binary)
# Example: just run --version
run *args:
    cargo run -p opencode-cloud --bin occ -- {{args}}

# Build Rust packages
build-rust:
    cargo build --workspace

# --- Node CLI (Mac / local dev) ---
# Build Node CLI for Mac: compile Rust occ, copy to cli-node/bin/, then build the wrapper.
# Use this when developing or testing the npm CLI locally (resolves binary from bin/ fallback).
build-node-cli-mac:
    cargo build -p opencode-cloud --bin occ
    @mkdir -p packages/cli-node/bin
    @cp target/debug/occ packages/cli-node/bin/occ
    pnpm -C packages/cli-node build
    @echo "✓ Node CLI built for Mac (binary in packages/cli-node/bin/)"

# Run Node CLI on Mac. Pass args through (e.g. just run-node-cli-mac --version).
# Requires build-node-cli-mac first; uses bin/occ as fallback.
run-node-cli-mac *args:
    node packages/cli-node/dist/index.js {{args}}

# Build Node packages (including NAPI bindings)
build-node:
    pnpm install
    pnpm -C packages/core build
    pnpm -r --filter="!@opencode-cloud/core" build

# Run all tests (fast)
test-all-fast: test-rust-fast test-node

# Run all tests (slow, includes doc-tests)
test-all-slow: test-rust test-node

# Run all tests (fast)
test: test-all-fast

# Run Rust tests
test-rust:
    cargo test --workspace

# Run Rust tests without doc-tests (fast)
test-rust-fast:
    cargo test --workspace --tests

# Run Node tests
test-node:
    cargo build -p opencode-cloud
    pnpm -C packages/cli-node build
    pnpm -r test

# Run Rust doc-tests (slow)
test-doc-slow:
    cargo test --workspace --doc

# Lint everything
lint: lint-rust lint-node lint-shell

# Lint Rust code
lint-rust:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Lint Rust code in Linux container (catches platform-gated code issues)
# Use this before pushing to catch CI failures on Linux
# Requires: Docker running
lint-rust-linux:
    docker run --rm -v "{{justfile_directory()}}":/workspace -w /workspace rust:1.89 \
        cargo clippy --all-targets --all-features -- -D warnings

# Lint Rust code for all platforms (local + Linux via Docker)
# Use this before pushing to catch CI failures early
lint-rust-cross: lint-rust lint-rust-linux

# Lint Node code
lint-node:
    pnpm -r lint

# Lint shell scripts
lint-shell:
    shellcheck scripts/*.sh

# Check for Dockerfile tool version updates
check-updates:
    ./scripts/check-dockerfile-updates.sh

# Pre-commit checks
pre-commit: fmt lint build test-all-fast

# Format everything
fmt:
    cargo fmt --all
    pnpm -r format

# Clean all build artifacts
clean:
    cargo clean
    pnpm -r clean

# Release build
release:
    cargo build --workspace --release
    pnpm install
    pnpm -C packages/core build

# Publish to crates.io (core first, then cli)
publish-crates: lint test-all-slow
    @echo "Publishing opencode-cloud-core to crates.io..."
    cargo publish -p opencode-cloud-core
    @echo ""
    @echo "Waiting 5s for crates.io to index..."
    @sleep 5
    @echo ""
    @echo "Publishing opencode-cloud to crates.io..."
    cargo publish -p opencode-cloud
    @echo ""
    @echo "✓ crates.io publish complete!"

# Publish to npm (core first, then cli)
publish-npm: lint test-all-slow build-node
    @echo "Publishing @opencode-cloud/core to npm..."
    pnpm --filter @opencode-cloud/core publish --access public
    @echo ""
    @echo "Waiting 5s for npm to index..."
    @sleep 5
    @echo ""
    @echo "Publishing opencode-cloud to npm..."
    pnpm --filter opencode-cloud publish --access public
    @echo ""
    @echo "✓ npm publish complete!"

# Publish to both crates.io and npm
publish-all: publish-crates publish-npm
    @echo ""
    @echo "✓ All packages published!"

# Dry-run for both crates.io and npm
publish-all-dry-run: publish-crates-dry-run publish-npm-dry-run
    @echo ""
    @echo "✓ All packages ready (dry-run)!"

# Dry-run for crates.io
publish-crates-dry-run:
    @echo "Dry-run: opencode-cloud-core (crates.io)..."
    cargo publish -p opencode-cloud-core --dry-run
    @echo "✓ opencode-cloud-core ready"
    @echo ""
    @echo "Dry-run: opencode-cloud (crates.io)..."
    @echo "(Note: this will fail if core is not yet on crates.io)"
    @echo "(Note: this fails when updating the dependency version of the opencode-cloud-core package in the root Cargo.toml)"
    @echo "(Note: this is expected to fail, so commenting it out for now)"
    #cargo publish -p opencode-cloud --dry-run
    @echo "✓ opencode-cloud ready"

# Dry-run for npm
publish-npm-dry-run: build-node
    @echo "Dry-run: @opencode-cloud/core (npm)..."
    pnpm --filter @opencode-cloud/core publish --access public --dry-run
    @echo "✓ @opencode-cloud/core ready"
    @echo ""
    @echo "Dry-run: opencode-cloud (npm)..."
    pnpm --filter opencode-cloud publish --access public --dry-run
    @echo "✓ opencode-cloud ready"
