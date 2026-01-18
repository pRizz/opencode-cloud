# justfile - Root task orchestration for opencode-cloud

# Default recipe
default: list

# List available recipes
list:
    @just --list

# Build everything
build: build-rust build-node

# Build Rust packages
build-rust:
    cargo build --workspace

# Build Node packages (including NAPI bindings)
build-node:
    pnpm -C packages/core build
    pnpm -r --filter="!@opencode-cloud/core" build

# Run all tests
test: test-rust test-node

# Run Rust tests
test-rust:
    cargo test --workspace

# Run Node tests
test-node:
    pnpm -r test

# Lint everything
lint: lint-rust lint-node

# Lint Rust code
lint-rust:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Lint Node code
lint-node:
    pnpm -r lint

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
    pnpm -C packages/core build

# Publish to crates.io (core first, then cli)
publish: lint test
    @echo "Publishing opencode-cloud-core..."
    cargo publish -p opencode-cloud-core
    @echo ""
    @echo "Waiting 30s for crates.io to index..."
    @sleep 30
    @echo ""
    @echo "Publishing opencode-cloud..."
    cargo publish -p opencode-cloud
    @echo ""
    @echo "✓ Published successfully!"

# Publish dry-run (verify without uploading)
publish-dry-run:
    @echo "Dry-run: opencode-cloud-core..."
    cargo publish -p opencode-cloud-core --dry-run
    @echo "✓ opencode-cloud-core ready"
    @echo ""
    @echo "Dry-run: opencode-cloud..."
    @echo "(Note: this will fail if core is not yet on crates.io)"
    cargo publish -p opencode-cloud --dry-run
    @echo "✓ opencode-cloud ready"
