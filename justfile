# justfile - Root task orchestration for opencode-cloud

# Default recipe
default: list

# List available recipes
list:
    @just --list

# Setup development environment (run once after cloning)
setup:
    git config core.hooksPath .githooks
    git -c url."https://github.com/".insteadOf=git@github.com: submodule update --init --recursive packages/opencode
    @command -v bun >/dev/null 2>&1 || (echo "Error: bun is required for this repo. Install from https://bun.sh and rerun just setup." && exit 1)
    bun install --cwd packages/opencode --frozen-lockfile
    bun install
    @echo "✓ Development environment ready!"
    @echo "Next: just dev"

# Build everything
build: build-rust build-node build-opencode build-opencode-broker

# Warn if packages/opencode commit differs from Dockerfile OPENCODE_COMMIT pin.
warn-opencode-pin-drift:
    @if [ ! -f packages/opencode/.git ] && [ ! -d packages/opencode/.git ]; then \
        :; \
    else \
        dockerfile="packages/core/src/docker/Dockerfile"; \
        dockerfile_pinned_commit="$(grep -oE 'OPENCODE_COMMIT=\"[^\"]+\"' "$dockerfile" | head -n1 | sed -E 's/OPENCODE_COMMIT=\"([^\"]+)\"/\1/')"; \
        submodule_commit="$(git -C packages/opencode rev-parse HEAD 2>/dev/null || true)"; \
        if [ -n "$dockerfile_pinned_commit" ] && [ -n "$submodule_commit" ] && [ "$dockerfile_pinned_commit" != "$submodule_commit" ]; then \
            echo "Warning: Dockerfile OPENCODE_COMMIT pin ($dockerfile_pinned_commit) differs from packages/opencode ($submodule_commit)."; \
            echo "         Run: just update-opencode-commit"; \
        fi; \
    fi

# Compile and run the occ binary (arguments automatically get passed to the binary)
# Example: just run --version
run *args: warn-opencode-pin-drift
    cargo run -p opencode-cloud --bin occ -- {{args}}

# Start occ with local opencode submodule and cached sandbox image rebuild.
dev: warn-opencode-pin-drift
    just run start --yes --local-opencode-submodule --cached-rebuild-sandbox-image

# --- Shared check/build base targets (used by local + CI wrappers) ---

check-rust-format:
    cargo fmt --all -- --check

check-rust-clippy:
    cargo clippy --all-targets --all-features -- -D warnings

build-rust-workspace:
    cargo build --workspace

verify-cli-version:
    cargo run -p opencode-cloud -- --version

build-core-bindings:
    bun run --cwd packages/core build

check-opencode-stack: lint-opencode build-opencode lint-opencode-broker

test-slow-suite: test-all-slow

# Build Rust packages
build-rust: build-rust-workspace

# --- Node CLI (Mac / local dev) ---
# Build Node CLI for Mac: compile Rust occ, copy to cli-node/bin/, then build the wrapper.
# Use this when developing or testing the npm CLI locally (resolves binary from bin/ fallback).
build-node-cli-mac:
    cargo build -p opencode-cloud --bin occ
    @mkdir -p packages/cli-node/bin
    @cp target/debug/occ packages/cli-node/bin/occ
    bun run --cwd packages/cli-node build
    @echo "✓ Node CLI built for Mac (binary in packages/cli-node/bin/)"

# Run Node CLI on Mac. Pass args through (e.g. just run-node-cli-mac --version).
# Requires build-node-cli-mac first; uses bin/occ as fallback.
run-node-cli-mac *args:
    node packages/cli-node/dist/index.js {{args}}

# Build Node packages (including NAPI bindings)
build-node:
    bun install
    bun run --cwd packages/core build
    bun run --cwd packages/cli-node build

# --- opencode Submodule Checks ---

# Ensure opencode submodule is initialized in this worktree
opencode-submodule-check:
    @if [ -f packages/opencode/.git ] || [ -d packages/opencode/.git ]; then \
        :; \
    else \
        echo "Submodule packages/opencode is not initialized."; \
        echo "Run: git submodule update --init --recursive"; \
        exit 1; \
    fi

# Install opencode dependencies when missing
opencode-install-if-needed: opencode-submodule-check
    @if [ ! -d packages/opencode/node_modules ]; then \
        echo "Installing opencode submodule dependencies..."; \
        bun install --cwd packages/opencode --frozen-lockfile; \
    else \
        echo "opencode submodule dependencies already installed."; \
    fi

# Typecheck opencode workspace
# Keep scripts.typecheck defined in each fork-* package so Turbo executes its task.
lint-opencode: opencode-install-if-needed check-fork-typecheck-wiring
    bun --cwd packages/opencode turbo typecheck

# Verify every fork-* package exposes scripts.typecheck for Turbo wiring
check-fork-typecheck-wiring: opencode-submodule-check
    ./scripts/check-fork-typecheck-wiring.sh

# Build the shared app package
build-opencode-app: opencode-install-if-needed
    bun run --cwd packages/opencode/packages/app build

# Build opencode single-ui artifact using local models fixture for deterministic output
build-opencode-single-ui: opencode-install-if-needed
    @tmpfile="$(mktemp)"; \
        trap 'rm -f "$tmpfile"' EXIT; \
        perl -pe 'chomp if eof' "{{justfile_directory()}}/packages/opencode/packages/opencode/test/tool/fixtures/models-api.json" > "$tmpfile"; \
        MODELS_DEV_API_JSON="$tmpfile" bun run --cwd packages/opencode/packages/opencode build-single-ui

# Smoke-check compiled opencode binary startup.
smoke-opencode-compiled: build-opencode-single-ui
    bun run --cwd packages/opencode/packages/opencode smoke:compiled

# Build opencode app and opencode binary/ui artifact
build-opencode: build-opencode-app build-opencode-single-ui smoke-opencode-compiled

# Lint opencode-broker Rust crate
lint-opencode-broker: opencode-submodule-check
    cargo fmt --manifest-path packages/opencode/packages/opencode-broker/Cargo.toml --all -- --check
    cargo clippy --manifest-path packages/opencode/packages/opencode-broker/Cargo.toml --all-targets -- -D warnings

# Build opencode-broker Rust crate
build-opencode-broker: opencode-submodule-check
    cargo build --manifest-path packages/opencode/packages/opencode-broker/Cargo.toml

# Test opencode-broker Rust crate
test-opencode-broker: opencode-submodule-check
    cargo test --manifest-path packages/opencode/packages/opencode-broker/Cargo.toml

# Run e2e tests (boots server in-process, seeds data, runs Playwright)
e2e: opencode-install-if-needed
    bun run --cwd packages/opencode/packages/app test:e2e:local

# Optional app unit test gate (not part of default pre-commit)
test-opencode-ui: opencode-install-if-needed
    bun run --cwd packages/opencode/packages/app test:unit

# Run opencode upstream unit tests (turbo: opencode + fork-tests + app)
# `--only` intentionally avoids Turbo dependency graph tasks (like `^build`)
# because CI build coverage for opencode already runs via lint/build targets.
test-opencode-unit: opencode-install-if-needed
    bun --cwd packages/opencode turbo test --only

# Debug path: include Turbo dependency build graph (`^build`) during tests.
test-opencode-unit-with-build: opencode-install-if-needed
    bun --cwd packages/opencode turbo test

# Optional submodule drift and dirty state check
check-opencode-submodule-drift:
    git submodule status --recursive
    git submodule foreach --recursive 'git status --short --branch'

# Ensure pinned opencode submodule commit is remotely fetchable
check-opencode-submodule-published:
    ./scripts/check-opencode-submodule-published.sh --from-index

# Update opencode submodule + Dockerfile OPENCODE_COMMIT pin
update-opencode-commit:
    ./scripts/update-opencode-commit.sh

# Format opencode-broker Rust crate
fmt-opencode-broker: opencode-submodule-check
    cargo fmt --manifest-path packages/opencode/packages/opencode-broker/Cargo.toml --all

# --- Docker Sandbox Image ---

# Build Docker sandbox image with BuildKit caching (amd64 only, for local dev)
# Use DOCKER_BUILDKIT=1 for layer caching and faster rebuilds
# The image is tagged as opencode-cloud-sandbox:dev
build-docker:
    @echo "Building Docker sandbox image..."
    @cp packages/core/src/docker/Dockerfile Dockerfile.build
    DOCKER_BUILDKIT=1 docker build \
        -f Dockerfile.build \
        -t opencode-cloud-sandbox:dev \
        --build-arg BUILDKIT_INLINE_CACHE=1 \
        .
    @rm -f Dockerfile.build
    @echo "✓ Docker image built: opencode-cloud-sandbox:dev"

# Build Docker sandbox image with no cache (clean rebuild)
build-docker-no-cache:
    @echo "Building Docker sandbox image (no cache)..."
    @cp packages/core/src/docker/Dockerfile Dockerfile.build
    DOCKER_BUILDKIT=1 docker build \
        -f Dockerfile.build \
        -t opencode-cloud-sandbox:dev \
        --no-cache \
        .
    @rm -f Dockerfile.build
    @echo "✓ Docker image built (no cache): opencode-cloud-sandbox:dev"

# Verify Docker build stages (builds opencode-build stage only, faster than full build)
check-docker:
    @echo "Checking Dockerfile syntax and build stages..."
    @cp packages/core/src/docker/Dockerfile Dockerfile.build
    DOCKER_BUILDKIT=1 docker build \
        -f Dockerfile.build \
        --target opencode-build \
        -t opencode-cloud-sandbox:check \
        .
    @rm -f Dockerfile.build
    @docker rmi opencode-cloud-sandbox:check 2>/dev/null || true
    @echo "✓ Dockerfile check passed"

# Run all tests (fast)
test-all-fast: test-rust-fast test-node test-opencode-fork-tests test-opencode-broker test-opencode-unit

# Run all tests (slow, includes doc-tests)
test-all-slow: test-rust test-node test-opencode-fork-tests test-opencode-broker test-opencode-unit

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
    bun run --cwd packages/cli-node build
    bun run --cwd packages/cli-node test

# Run opencode fork tests (Bun workspace under submodule)
test-opencode-fork-tests: opencode-install-if-needed
    OPENCODE_CONFIG_CONTENT='{"auth":{"enabled":false}}' bun test --cwd packages/opencode/packages/fork-tests

# Run Rust doc-tests (slow)
test-doc-slow:
    cargo test --workspace --doc

# Lint everything
lint: lint-rust lint-node lint-shell lint-workflows lint-opencode lint-opencode-broker

# Lint Rust code
lint-rust: check-rust-format check-rust-clippy

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
    bun --workspaces --if-present run lint

# Lint shell scripts
lint-shell:
    shellcheck scripts/*.sh

# Lint GitHub Actions workflows (root repo only; shellcheck disabled — covered by lint-shell)
lint-workflows:
    actionlint -shellcheck= -pyflakes= .github/workflows/*.yml

# --- CI wrappers (CI install remains stricter than local by design) ---

ci-node-install:
    bun install --frozen-lockfile --ignore-scripts

ci-node-install-cli-only:
    bun install --filter opencode-cloud --frozen-lockfile --ignore-scripts

# Keep browser install colocated with `ci-e2e` execution and use
# PLAYWRIGHT_BROWSERS_PATH=0 by default to pin browser binaries to Bun's
# installed playwright-core version.
ci-e2e *args:
    bun install --cwd packages/opencode --frozen-lockfile --ignore-scripts
    @set -- {{args}}; \
    if [ "${1:-}" = "--" ]; then shift; fi; \
    models_path="${OPENCODE_MODELS_PATH:-{{justfile_directory()}}/packages/opencode/packages/opencode/test/tool/fixtures/models-api.json}"; \
    if [ ! -f "$models_path" ]; then \
        echo "Error: OPENCODE_MODELS_PATH file does not exist: $models_path"; \
        exit 1; \
    fi; \
    playwright_browsers_path="${PLAYWRIGHT_BROWSERS_PATH:-0}"; \
    if [ "${CI:-}" = "true" ] || [ "${CI:-}" = "1" ]; then \
        PLAYWRIGHT_BROWSERS_PATH="$playwright_browsers_path" ./packages/opencode/packages/app/node_modules/.bin/playwright install --with-deps chromium; \
    else \
        PLAYWRIGHT_BROWSERS_PATH="$playwright_browsers_path" ./packages/opencode/packages/app/node_modules/.bin/playwright install chromium; \
    fi; \
    CI="${CI:-true}" OPENCODE_DISABLE_MODELS_FETCH="${OPENCODE_DISABLE_MODELS_FETCH:-true}" OPENCODE_MODELS_PATH="$models_path" PLAYWRIGHT_BROWSERS_PATH="$playwright_browsers_path" bun run --cwd packages/opencode/packages/app test:e2e:local -- "$@"

ci-lint: lint-rust check-opencode-stack

ci-build: build-rust build-core-bindings

ci-test: test-slow-suite

ci-verify: verify-cli-version

ci-checks: ci-lint ci-build ci-test ci-verify

# Check for Dockerfile tool version updates
check-updates:
    ./scripts/check-dockerfile-updates.sh

# --- DigitalOcean Marketplace ---

# Validate the DigitalOcean Marketplace Packer template
do-marketplace-validate:
    packer init infra/digitalocean/packer/opencode-marketplace.pkr.hcl
    packer fmt -check infra/digitalocean/packer/opencode-marketplace.pkr.hcl
    packer validate -var-file=infra/digitalocean/packer/variables.pkr.hcl \
        infra/digitalocean/packer/opencode-marketplace.pkr.hcl

# Build the DigitalOcean Marketplace snapshot
do-marketplace-build:
    packer init infra/digitalocean/packer/opencode-marketplace.pkr.hcl
    packer build -var-file=infra/digitalocean/packer/variables.pkr.hcl \
        infra/digitalocean/packer/opencode-marketplace.pkr.hcl

# Pre-commit checks with conditional Docker stage build for Docker-risk changes.
# This keeps routine commits fast while still catching Docker context regressions.
pre-commit: check-opencode-submodule-published fmt lint build test-all-fast e2e
    @if ./scripts/should-run-docker-check.sh; then \
        echo "Running Docker stage check because Docker-risk files changed..."; \
        just check-docker; \
    else \
        echo "Skipping Docker stage check (no Docker-risk file changes)."; \
    fi

# Pre-commit checks including Docker build (requires Docker)
pre-commit-full: check-opencode-submodule-published fmt lint build test-all-fast e2e build-docker
    @echo "✓ Full pre-commit checks passed (including Docker build)"

# Format everything
fmt: fmt-opencode-broker
    cargo fmt --all
    bun --workspaces --if-present run format

# Clean all build artifacts
clean:
    cargo clean
    bun --workspaces --if-present run clean

# Release build
release:
    cargo build --workspace --release
    bun install
    bun run --cwd packages/core build

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
    bun publish --cwd packages/core --access public
    @echo ""
    @echo "Waiting 5s for npm to index..."
    @sleep 5
    @echo ""
    @echo "Publishing opencode-cloud to npm..."
    bun publish --cwd packages/cli-node --access public
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
    bun publish --cwd packages/core --access public --dry-run
    @echo "✓ @opencode-cloud/core ready"
    @echo ""
    @echo "Dry-run: opencode-cloud (npm)..."
    bun publish --cwd packages/cli-node --access public --dry-run
    @echo "✓ opencode-cloud ready"
