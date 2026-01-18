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
publish-crates: lint test
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
publish-npm: lint test build-node
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
    cargo publish -p opencode-cloud --dry-run
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
