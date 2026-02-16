# opencode-cloud

<!-- BEGIN:opencode-cloud-readme-badges -->
[![GitHub Stars](https://img.shields.io/github/stars/pRizz/opencode-cloud)](https://github.com/pRizz/opencode-cloud)
[![CI](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/pRizz/opencode-cloud/actions/workflows/ci.yml)
[![Mirror](https://img.shields.io/badge/mirror-gitea-blue?logo=gitea)](https://gitea.com/pRizz/opencode-cloud)
[![crates.io](https://img.shields.io/crates/v/opencode-cloud.svg)](https://crates.io/crates/opencode-cloud)
[![Crates Downloads](https://img.shields.io/crates/d/opencode-cloud.svg)](https://crates.io/crates/opencode-cloud)
[![npm Downloads](https://img.shields.io/npm/dt/opencode-cloud?logo=npm)](https://www.npmjs.com/package/opencode-cloud)
[![Docker Hub](https://img.shields.io/docker/v/prizz/opencode-cloud-sandbox?label=docker&sort=semver)](https://hub.docker.com/r/prizz/opencode-cloud-sandbox)
[![Docker Pulls](https://img.shields.io/docker/pulls/prizz/opencode-cloud-sandbox)](https://hub.docker.com/r/prizz/opencode-cloud-sandbox)
[![GHCR](https://img.shields.io/badge/ghcr.io-sandbox-blue?logo=github)](https://github.com/pRizz/opencode-cloud/pkgs/container/opencode-cloud-sandbox)
[![docs.rs](https://docs.rs/opencode-cloud/badge.svg)](https://docs.rs/opencode-cloud)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
<!-- END:opencode-cloud-readme-badges -->

> [!WARNING]
> This tool is still a work in progress and is rapidly evolving. Expect bugs, frequent updates, and breaking changes. Follow updates on [GitHub](https://github.com/pRizz/opencode-cloud) ([Gitea mirror](https://gitea.com/pRizz/opencode-cloud)) and [X (Twitter)](https://x.com/pryszkie). Stability will be announced at some point. Use with caution.

A production-ready toolkit for deploying and managing our [opencode fork](https://github.com/pRizz/opencode) (forked from [anomalyco/opencode](https://github.com/anomalyco/opencode)) as a persistent cloud service, **sandboxed inside a Docker container** for isolation and security.

This fork adds **passkey-first authentication** (WebAuthn/FIDO2), two-factor authentication, and enterprise security features for cloud deployment.

## Quick Deploy (Docker)

Deploy opencode-cloud with one command. Installs Docker if needed (Linux), downloads or refreshes the Docker Compose config, pulls the latest `prizz/opencode-cloud-sandbox:latest` image, reconciles services, and prints the login credentials:

```bash
curl -fsSL https://raw.githubusercontent.com/pRizz/opencode-cloud/main/scripts/quick-deploy.sh | bash
```

Then open [http://localhost:3000](http://localhost:3000) and enter the Initial One-Time Password (IOTP) to complete setup.

> **macOS/Windows:** Install [Docker Desktop](https://www.docker.com/products/docker-desktop/) first, then run the command above.

> **Remote server:** SSH into the server, run the command, then access via SSH tunnel: `ssh -L 3000:localhost:3000 root@<server-ip>`

> **Interactive mode:** Add `--interactive` to be prompted before each step: `curl -fsSL .../scripts/quick-deploy.sh | bash -s -- --interactive`

> **Compose refresh behavior:** By default, the script fetches the latest upstream `docker-compose.yml`. If your local file differs, it is replaced and a backup is written as `docker-compose.yml.bak.<timestamp>`.

> **Image refresh behavior:** By default, the script runs `docker compose pull` before `docker compose up -d`, so rerunning quick deploy updates to the latest image.

## Quick install (cargo)

```bash
cargo install opencode-cloud
opencode-cloud --version
```

## Quick install (npm)

```bash
npx opencode-cloud@latest --version
```

```bash
bunx opencode-cloud@latest --version
```

Or install globally:
```bash
npm install -g opencode-cloud
opencode-cloud --version
```

## Deploy to AWS

[![Deploy to AWS](https://s3.amazonaws.com/cloudformation-examples/cloudformation-launch-stack.png)](https://console.aws.amazon.com/cloudformation/home#/stacks/create/review?templateURL=https://opencode-cloud-templates.s3.us-east-2.amazonaws.com/cloudformation/opencode-cloud-quick.yaml)

Quick deploy provisions a private EC2 instance behind a public ALB with HTTPS.
**A domain name is required** for ACM certificate validation.
**A Route53 hosted zone ID is required** for automated DNS validation.

Docs: `docs/deploy/aws.md` (includes teardown steps and S3 hosting setup for forks)
Credentials: `docs/deploy/aws.md#retrieving-credentials`

## Deploy to Railway

<!-- TODO: Replace TEMPLATE_CODE with the actual Railway template code once the template is created. See docs/deploy/railway.md for template creation instructions. -->
[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/new/template/TEMPLATE_CODE)

One-click deploy provisions a Railway service with automatic HTTPS.

> **Important:** Attach a Railway Volume mounted to `/home/opencoder/.local/share/opencode` to prevent data loss across redeploys.
> For manual template import, use `docker-compose.railway-template-base.yml` as a Railway importer compatibility base (not as the canonical runtime compose).

Docs: `docs/deploy/railway.md`

## Run with Docker / Docker Desktop

> **Tip:** For a fully automated setup, see [Quick Deploy](#quick-deploy-docker) above.

The fastest way to run opencode-cloud locally:

```bash
docker compose up -d
```

This uses the included `docker-compose.yml` which configures all persistent volumes automatically.

Optional `.env` overrides (same directory as `docker-compose.yml`):

```bash
# Example: expose publicly and pin a reproducible image tag
cat > .env <<'EOF'
OPENCODE_PORT_MAPPING=3000:3000
OPENCODE_IMAGE=prizz/opencode-cloud-sandbox:15.2.0
EOF
```

By default, Compose uses `OPENCODE_PULL_POLICY=missing`. To force-refresh to newer image layers:

```bash
docker compose pull && docker compose up -d
```

Retrieve the Initial One-Time Password (IOTP) and open `http://localhost:3000`:

```bash
docker compose logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

Docs: `docs/deploy/docker-desktop.md`

## Deploy to DigitalOcean

### Marketplace (Coming Soon)

DigitalOcean Marketplace one-click deployment is in progress. Support is coming soon.

Docs: `docs/deploy/digitalocean-marketplace.md`

### Droplet (Manual)

SSH into an Ubuntu 24.04 Droplet and run:

```bash
curl -fsSL https://raw.githubusercontent.com/pRizz/opencode-cloud/main/scripts/quick-deploy.sh | bash
```

This installs Docker, by default refreshes the Compose file from upstream (with backup if your local copy differs), pulls the latest image, reconciles services, and prints the IOTP.

Access via SSH tunnel: `ssh -L 3000:localhost:3000 root@<droplet-ip>`, then open `http://localhost:3000`.

Docs: `docs/deploy/digitalocean-droplet.md`

## Features

- **Sandboxed execution** - opencode runs inside a Docker container, isolated from your host system
- **Passkey-first authentication** - WebAuthn/FIDO2 passkeys as the primary login method, with username/password and TOTP 2FA as fallback options
- **Persistent environment** - Your projects, settings, and shell history persist across restarts
- **Cross-platform CLI** (`opencode-cloud` / `occ`) - Works on Linux and macOS
- **Service lifecycle commands** - start, stop, restart, status, logs
- **Platform service integration** - systemd (Linux) / launchd (macOS) for auto-start on boot
- **Remote host management** - Manage opencode containers on remote servers via SSH

## How it works

opencode-cloud runs opencode inside a Docker container, providing:

- **Isolation** - opencode and its AI-generated code run in a sandbox, separate from your host system
- **Reproducibility** - The container includes a full development environment (languages, tools, runtimes)
- **Persistence** - Docker volumes preserve your work across container restarts and updates
- **Security** - Network exposure is opt-in; by default, the service only binds to localhost

The CLI manages the container lifecycle, so you don't need to interact with Docker directly.

## Docker Images

The sandbox container image is named **`opencode-cloud-sandbox`** (not `opencode-cloud`) to clearly distinguish it from the CLI tool. The preferred way to use and manage the image is via the opencode-cloud CLI ([GitHub](https://github.com/pRizz/opencode-cloud), mirror: https://gitea.com/pRizz/opencode-cloud). It handles image pulling, container setup, and upgrades for you.

**Why use the CLI?** It configures volumes, ports, and upgrades safely, so you don’t have to manage `docker run` flags or image updates yourself.

The image is published to both registries (Docker Hub is the primary distribution):

| Registry | Image |
|----------|-------|
| Docker Hub | [`prizz/opencode-cloud-sandbox`](https://hub.docker.com/r/prizz/opencode-cloud-sandbox) |
| GitHub Container Registry | [`ghcr.io/prizz/opencode-cloud-sandbox`](https://github.com/pRizz/opencode-cloud/pkgs/container/opencode-cloud-sandbox) |

Pull commands:

Docker Hub:
```bash
docker pull prizz/opencode-cloud-sandbox:latest
```

GitHub Container Registry:
```bash
docker pull ghcr.io/prizz/opencode-cloud-sandbox:latest
```

**For most users:** Just use the CLI - it handles image pulling/building automatically:
```bash
occ start  # Pulls or builds the image as needed
```

**Running the image directly** (without the CLI)? Use Docker Compose or configure named volumes for persistence. See `docs/deploy/docker-desktop.md` for Docker Desktop / `docker run`, or `docs/deploy/railway.md` for Railway.

## Requirements

- **Rust 1.85+** - Install via [rustup](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Docker** - For running the opencode container
- **Supported platforms** - Linux and macOS

## Installation

### Via cargo (recommended)

```bash
cargo install opencode-cloud
occ --version
```

### Via npm

```bash
npx opencode-cloud@latest --version
```

```bash
bunx opencode-cloud@latest --version
```

Or install globally:
```bash
npm install -g opencode-cloud
occ --version
```

## First run

```bash
# Install as a system service (recommended for background use)
occ install

# Start the service
occ start
```

If this is the first startup with no configured managed users, the container logs will print an **Initial One-Time Password (IOTP)**.
Open the login page, use the first-time setup panel, then continue to passkey setup where you can either enroll a passkey or choose username/password registration.
The IOTP is invalidated after successful passkey enrollment or successful username/password bootstrap signup.
Built-in image users (for example `ubuntu`/`opencoder`) do not count as configured users for IOTP bootstrap.

Quick IOTP extraction from logs:

```bash
occ logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

You can also run the setup wizard:

```bash
occ setup
```

The wizard now configures runtime settings (image source, bind/port, mounts), keeps authentication on IOTP-first onboarding, and attempts to auto-detect the IOTP from logs after start.

### From source (install locally)

```bash
# GitHub (primary)
git clone https://github.com/pRizz/opencode-cloud.git

# Gitea (mirror)
git clone https://gitea.com/pRizz/opencode-cloud.git
cd opencode-cloud
git submodule update --init --recursive packages/opencode
cargo install --path packages/cli-rust
```

### From source (development run)

```bash
# GitHub (primary)
git clone https://github.com/pRizz/opencode-cloud.git

# Gitea (mirror)
git clone https://gitea.com/pRizz/opencode-cloud.git
cd opencode-cloud
git submodule update --init --recursive packages/opencode
 
# Bun is required for this repo
bun --version

just setup
just build
just dev    # Recommended local dev start shortcut
cargo run -p opencode-cloud -- --version
```

## Usage

```bash
# Show version
occ --version

# Start the service (builds Docker container on first run, ~10-15 min)
occ start

# Start on a custom port
occ start --port 8080

# Start and open browser
occ start --open

# Check service status (includes broker health: Healthy/Degraded/Unhealthy)
occ status

# View logs
occ logs

# Follow logs in real-time
occ logs -f

# View opencode-broker logs (systemd/journald required)
occ logs --broker

# Troubleshoot broker health issues reported by `occ status`
occ logs --broker --no-follow

# Note: Broker logs require systemd/journald. This is enabled by default on supported Linux
# hosts. Docker Desktop/macOS/Windows use Tini, so broker logs aren't available there.
# Existing containers may need to be recreated after upgrading.

# Stop the service
occ stop

# Restart the service
occ restart

# Check for updates and choose what to update
occ update

# Update the opencode-cloud CLI binary
occ update cli

# Update the opencode-cloud container image
occ update container

# Update opencode inside the container
occ update opencode

# Update opencode using a specific branch or commit
occ update opencode --branch dev
occ update opencode --commit <sha>

# Remove the container (keeps volumes)
occ reset container

# Remove container and volumes (data loss)
occ reset container --volumes --force

# Reset completed IOTP bootstrap and generate a fresh one-time password
occ reset iotp

# If bind_address is exposed (for example behind HTTPS reverse proxy), confirm intentionally
occ reset iotp --force

# Clean bind mount contents (data loss)
occ mount clean --force

# Purge bind mounts (data loss, removes config entries)
occ mount clean --purge --force

# Mount a local project into the workspace
occ mount add /Users/<username>/Desktop/opencode:/home/opencoder/workspace

# Apply mount changes (you may be prompted to recreate the container)
occ restart

# Factory reset host (container, volumes, mounts, config/data)
occ reset host --force

### Workspace Mounts

Use `/home/opencoder/workspace` when you want your host project folder to appear in the
web UI's project picker and inside the container workspace.

Important behavior:
- `/home/opencoder/workspace` is a single mount target.
- Adding a new mount to this same target replaces the previous mount entry.

Recommended workflow:
```bash
occ mount add /Users/<username>/Desktop/opencode:/home/opencoder/workspace
occ restart
```

Verify the mount:
1. Run `occ status` and check `Mounts` -> `Bind mounts` includes your host path mapped to `/home/opencoder/workspace`.
2. In the web UI, open the project picker and confirm your project files appear under `~/workspace`.

### Container Mode

When `occ` runs inside the opencode container, it will auto-detect this and switch to **container runtime**.
Override if needed:

```bash
occ --runtime host <command>
OPENCODE_RUNTIME=host occ <command>
```

Supported commands in container runtime:
- `occ status`
- `occ logs`
- `occ user`
- `occ update opencode`

Notes:
- Host/Docker lifecycle commands are disabled in container runtime.
- `occ logs` and `occ update opencode` require systemd inside the container. If systemd is not available, run those commands from the host instead.

### Webapp-triggered update (command file)

When running in foreground mode (for example via `occ install`, which uses `occ start --no-daemon`),
the host listens for a command file on a bind mount. The webapp can write a simple JSON payload
to request an update.

Default paths (with default bind mounts enabled):
- Host: `~/.local/state/opencode/opencode-cloud/commands/update-command.json`
- Container: `/home/opencoder/.local/state/opencode/opencode-cloud/commands/update-command.json`

Example payload:
```json
{
  "command": "update_opencode",
  "request_id": "webapp-1234",
  "branch": "dev"
}
```

The host writes the result to:
`~/.local/state/opencode/opencode-cloud/commands/update-command.result.json`

# Install as a system service (starts on login/boot)
occ install

# Uninstall the system service
occ uninstall

# View configuration
occ config show
```

## Authentication

opencode-cloud uses **passkey-first authentication** — WebAuthn/FIDO2 passkeys are the primary login method, providing phishing-resistant, passwordless sign-in. Username/password (via PAM) and TOTP two-factor authentication are available as fallback options.

Security details: `docs/security/passkey-registration.md`

First boot path:
- If no managed users are configured, startup logs print an Initial One-Time Password (IOTP).
- Extract only the IOTP quickly: `occ logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'`
- `occ setup` attempts to auto-detect and print the IOTP after starting/restarting the service.
- Enter that IOTP in the web login page first-time setup panel.
- Continue to passkey setup, then choose one of:
  - Enroll a passkey for the default `opencoder` account, or
  - Use the username/password fallback to create your first managed user.
- The IOTP is deleted after successful passkey enrollment or successful username/password bootstrap signup.
- To restart first-time onboarding after completion, run `occ reset iotp`.
- Built-in image users (for example `ubuntu`/`opencoder`) do not disable IOTP bootstrap.

Admin path:
- You can always create/manage users directly via `occ user add`, `occ user passwd`, and related user commands.

Login UX:
- **Passkey sign-in is front and center** — the login page leads with WebAuthn for fast, phishing-resistant authentication.
- Username/password sign-in remains available as a fallback.
- TOTP two-factor authentication can be enabled per-user from the session menu after login.

### Creating Users

Create a user with a password:
```bash
occ user add <username>
```

Generate a random password:
```bash
occ user add <username> --generate
```

### Managing Users

- List users: `occ user list` (managed users only)
- Change password: `occ user passwd <username>`
- Remove user: `occ user remove <username>`
- Enable/disable account: `occ user enable <username>` / `occ user disable <username>`

### User Persistence

User accounts (including password hashes and lock status) persist across container updates and rebuilds.
The CLI stores user records in a managed Docker volume mounted at `/var/lib/opencode-users` inside the container.
This record store is the source of truth for configured users and bootstrap decisions.
No plaintext passwords are stored on the host.

### Rebuilding the Docker Image

When developing locally or after updating opencode-cloud, you may need to rebuild the Docker image to pick up changes in the embedded Dockerfile:

```bash
# Rebuild using Docker cache (fast - only rebuilds changed layers)
occ start --cached-rebuild-sandbox-image

# Rebuild from scratch without cache (slow - for troubleshooting)
occ start --full-rebuild-sandbox-image
```

**`--cached-rebuild-sandbox-image`** (recommended for most cases):
- Uses Docker layer cache for fast rebuilds
- Only rebuilds layers that changed (e.g., if only the CMD changed, it's nearly instant)
- Stops and removes any existing container before rebuilding

**`--full-rebuild-sandbox-image`** (for troubleshooting):
- Ignores Docker cache and rebuilds everything from scratch
- Takes 10-15 minutes but guarantees a completely fresh image
- Use when cached rebuild doesn't fix issues

**When to rebuild:**
- After pulling updates to opencode-cloud → use `--cached-rebuild-sandbox-image`
- After pulling new commits in `packages/opencode` (submodule) → run `just run start --cached-rebuild-sandbox-image` once so the running container picks up the new opencode commit
- When modifying the Dockerfile during development → use `--cached-rebuild-sandbox-image`
- When the container fails to start due to image issues → try `--cached-rebuild-sandbox-image` first, then `--full-rebuild-sandbox-image`
- When you want a completely fresh environment → use `--full-rebuild-sandbox-image`

**Local submodule dev rebuild (no push required):**
```bash
# Recommended shortcut for fast rebuild using local packages/opencode checkout (including uncommitted edits)
just dev

# Equivalent long form:
just run start --yes --local-opencode-submodule --cached-rebuild-sandbox-image

# Full no-cache rebuild from local packages/opencode checkout
just run start --full-rebuild-sandbox-image --local-opencode-submodule
```
- This mode is for local development/debugging only and bypasses the default remote clone path.
- `just run status` will show commit metadata derived from your local checkout (dirty trees are marked with `-dirty`).
- Local context packaging intentionally skips heavyweight/dev metadata folders (for example `.planning`, `.git`, `node_modules`, `target`, and `dist`).
- Keep CI/release workflows on the default pinned remote mode.

### Dockerfile Optimization Checklist

For new Docker build steps, follow this checklist:
- Prefer BuildKit cache mounts (`RUN --mount=type=cache`) for package caches (`apt`, `bun`, `cargo`, `pip`, and `npm`).
- For `bun install` in container builds, use a dedicated install-cache mount plus a short retry loop that clears that cache between attempts to recover from occasional corrupted/interrupted cache artifacts.
- Create and remove temporary workdirs in the same `RUN` layer (for example `/tmp/opencode-repo`).
- Do not defer cleanup to later layers; deleted files still exist in lower layers.
- Keep builder-stage artifacts out of runtime layers by copying only final outputs.
- When adding Docker build assets, update build-context inclusion logic in `packages/core/src/docker/image.rs`.
- Keep local submodule exclude lists aligned with Dockerfile hygiene rules (`.planning`, `.git`, `node_modules`, `target`, `dist`, and similar dev metadata).

This policy is intentional for both image-size hygiene and fast local rebuilds.

**Worktree-isolated sandbox profiles (opt-in):**
```bash
# Derive a stable instance name from the current git worktree root
just run --sandbox-instance auto start --cached-rebuild-sandbox-image

# Use an explicit instance name
just run --sandbox-instance mytree start --cached-rebuild-sandbox-image
```
- Default behavior (no `--sandbox-instance`) remains the shared legacy sandbox.
- Isolated instances use separate container names, image tags, Docker volumes, and image-state files.
- You can also set `OPENCODE_SANDBOX_INSTANCE=<name|auto>` instead of passing the CLI flag every time.

## Configuration

Configuration is stored at:
- Linux/macOS: `~/.config/opencode-cloud/config.json`

Data (PID files, etc.) is stored at:
- Linux/macOS: `~/.local/share/opencode-cloud/`

## Development

### Prerequisites

- **Rust 1.85+** — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Bun 1.3.9+** — `curl -fsSL https://bun.sh/install | bash`
- **just** (task runner) — `cargo install just` or `brew install just`
- **Docker** — [Docker Desktop](https://www.docker.com/products/docker-desktop/) or Docker Engine
- **Node.js 20+** — for the Node CLI wrapper

```bash
# One-time setup (hooks + deps + submodule bootstrap)
just setup

# Build everything
just build

# Recommended local dev runtime (local submodule + cached sandbox rebuild)
just dev

# Compile and run occ (arguments automatically get passed to the binary)
just run --version

# Run tests
just test

# Format and lint
just fmt
just lint
```

### Visual E2E (Playwright)

From the repository root, run UI tests in a visible browser with:

```bash
bun run --cwd packages/opencode/packages/app test:e2e:local -- --headed --project=chromium e2e/settings/settings-authentication.spec.ts
bun run --cwd packages/opencode/packages/app test:e2e:local -- --ui e2e/settings/settings-authentication.spec.ts
PWDEBUG=1 bun run --cwd packages/opencode/packages/app test:e2e:local -- --headed --project=chromium e2e/settings/settings-authentication.spec.ts
```

Headless run (default Playwright mode):

```bash
bun run --cwd packages/opencode/packages/app test:e2e:local -- --project=chromium e2e/settings/settings-authentication.spec.ts
```

Use `test:e2e:local` for local harness/sandbox provisioning. Plain `test:e2e` expects a backend that is already running at the configured Playwright host/port.

> **Note:** The git hooks automatically sync `README.md` to npm package directories on commit.

## Architecture

This is a monorepo with:
- `packages/core` - Rust core library
- `packages/cli-rust` - Rust CLI binary (recommended)
- `packages/cli-node` - Node.js CLI (fully supported and in parity with the Rust CLI)

### Cargo.toml Sync Requirement

The `packages/core/Cargo.toml` file must use **explicit values** rather than `workspace = true` references.

When updating package metadata (version, edition, rust-version, etc.), keep both files in sync:
- `Cargo.toml` (workspace root)
- `packages/core/Cargo.toml`

Use `scripts/set-all-versions.sh <version>` to update versions across all files automatically.

## License

MIT
