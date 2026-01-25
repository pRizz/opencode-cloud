# Roadmap: opencode-cloud

## Overview

This roadmap delivers a cross-platform CLI toolkit for deploying opencode as a persistent cloud service. Starting with project foundation and CLI skeletons, we build up Docker management, service lifecycle commands, platform-specific service installation, an interactive setup wizard, security/authentication, update capabilities, and finish with polish and documentation. Each phase delivers a coherent, testable capability that builds toward the core value: developers accessing a persistent, secure opencode instance from anywhere.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Project Foundation** - Monorepo structure, CLI skeletons, config management
- [x] **Phase 2: Docker Integration** - Container lifecycle via Bollard
- [x] **Phase 3: Service Lifecycle Commands** - Start/stop/restart/status/logs
- [x] **Phase 4: Platform Service Installation** - systemd/launchd registration, boot persistence
- [x] **Phase 5: Interactive Setup Wizard** - Guided first-run experience
- [x] **Phase 6: Security and Authentication** - PAM-based auth, localhost binding, network exposure opt-in
- [x] **Phase 7: Update and Maintenance** - Update command, health check endpoint
- [x] **Phase 8: Polish and Documentation** - Help docs, error messages, uninstall cleanup
- [x] **Phase 9: Dockerfile Version Pinning** - Pin explicit versions for GitHub-installed tools
- [x] **Phase 10: Remote Administration via Cockpit** - Integrate and expose remote admin of Docker container via Cockpit
- [x] **Phase 11: Remote Host Management** - Allow occ to remotely install and interact with Docker containers on different hosts
- [~] **Phase 12: Web Desktop UI Investigation** - ~~Investigate integrating secure web-exposed desktop UI~~ (DEFERRED)
- [~] **Phase 13: Container Security Tools** - ~~Add trivy, gitleaks, hadolint, age, sops, mkcert to container~~ (DEFERRED)
- [x] **Phase 14: Versioning and Release Automation** - CI/CD for Docker images, version detection, auto-rebuild prompts
- [x] **Phase 15: Prebuilt Image Option** - Option to pull prebuilt images vs building from scratch
- [x] **Phase 16: Code Quality Audit** - Reduce nesting, eliminate duplication, improve readability
- [x] **Phase 17: Custom Bind Mounts** - Allow users to mount local directories into the container
- [x] **Phase 18: CLI Sync Strategy** - Strategy for keeping Rust and Node CLIs in sync
- [~] **Phase 19: CI/CD Automation** - ~~Automated Docker image uploads~~ (MERGED into Phase 14)
- [ ] **Phase 20: One-Click Cloud Deploy** - Deploy buttons for AWS, GCP, Azure etc. that provision cloud instances with opencode-cloud pre-installed
- [ ] **Phase 21: Use opencode Fork with PAM Authentication** - Switch to pRizz/opencode fork for proper PAM-based web authentication
- [ ] **Phase 22: Dedupe CLI Logic** - Consolidate CLI so Rust is single source of truth, Node delegates
- [ ] **Phase 23: Container Shell Access** - `occ shell` command for quick terminal access to running container
- [ ] **Phase 24: IDE Integration** - VS Code and JetBrains extensions to connect to container
- [ ] **Phase 25: Container Templates** - Pre-configured environment templates (Python ML, Node.js, Rust, etc.)
- [ ] **Phase 26: Secrets Management** - Secure injection of API keys and credentials into container
- [ ] **Phase 27: Windows Support** - Full Windows compatibility for the CLI and Docker integration
- [ ] **Phase 28: Remote Host Setup Wizard** - Run setup wizard for remote hosts via --host flag
- [ ] **Phase 29: DockerHub README Elaboration** - Elaborate README.dockerhub.md for better discoverability
- [ ] **Phase 30: Deep Nesting Refactor** - Replace deep nesting with early returns, guard clauses, let-else, and helper functions

## Phase Details

### Phase 1: Project Foundation
**Goal**: Establish monorepo structure with working npm and cargo CLI entry points that can read/write configuration
**Depends on**: Nothing (first phase)
**Requirements**: INST-01, CONF-04, CONF-07, CONS-01
**Success Criteria** (what must be TRUE):
  1. User can install via `npx opencode-cloud --version` and see version output (compiles from source; Rust 1.85+ required)
  2. User can install via `cargo install` (from local path) and run `opencode-cloud --version` (compiles from source)
  3. Configuration file is created at platform-appropriate XDG-compliant path
  4. Configuration file format matches documented JSON schema
  5. Only one instance can run per host (singleton enforcement)
**Plans**: 2 plans

Plans:
- [x] 01-01-PLAN.md — Monorepo structure, Rust/Node workspaces, CLI skeletons with version output
- [x] 01-02-PLAN.md — Config management (XDG paths, JSONC, schema), singleton enforcement

### Phase 2: Docker Integration
**Goal**: CLI can build/pull our custom opencode image and manage container lifecycle programmatically
**Depends on**: Phase 1
**Requirements**: PERS-02, PERS-03, PERS-04
**Note**: We supply our own Dockerfile based on Ubuntu for complete control. Base image is hardcoded for v1; user-configurable base image (Alpine, Debian, etc.) deferred to future version.
**Success Criteria** (what must be TRUE):
  1. CLI can build or pull the custom opencode Docker image with progress indicator
  2. CLI can create and start a container running opencode web UI
  3. CLI can stop and remove the container
  4. Session history, project files, and configuration persist in Docker volumes across container restarts
**Plans**: 3 plans

Plans:
- [x] 02-01-PLAN.md — Docker client wrapper (Bollard), error types, embedded Dockerfile
- [x] 02-02-PLAN.md — Image build/pull operations with progress feedback
- [x] 02-03-PLAN.md — Volume persistence and container lifecycle (create/start/stop/remove)

### Phase 3: Service Lifecycle Commands
**Goal**: User can control the service through intuitive CLI commands
**Depends on**: Phase 2
**Requirements**: LIFE-01, LIFE-02, LIFE-03, LIFE-04, LIFE-05
**Success Criteria** (what must be TRUE):
  1. User can start the service via `opencode-cloud start`
  2. User can stop the service via `opencode-cloud stop`
  3. User can restart the service via `opencode-cloud restart`
  4. User can check status via `opencode-cloud status` and see running/stopped state
  5. User can view logs via `opencode-cloud logs` with follow mode (`-f`)
**Plans**: 2 plans

Plans:
- [x] 03-01-PLAN.md — Start/stop/restart commands with spinner feedback and idempotent behavior
- [x] 03-02-PLAN.md — Status and logs commands with colored output and log streaming

### Phase 4: Platform Service Installation
**Goal**: Service survives host reboots and auto-restarts on crash
**Depends on**: Phase 3
**Requirements**: PLAT-01, PLAT-02, PERS-01, PERS-05, PERS-06
**Success Criteria** (what must be TRUE):
  1. On Linux, service is registered with systemd and starts on boot
  2. On macOS, service is registered with launchd and starts on login
  3. Service automatically restarts after crash (within configured retry limits)
  4. User can configure restart policies (retry count, delay between retries)
  5. Service unit files are placed in user-level directories (no root required)
**Plans**: 4 plans

Plans:
- [x] 04-01-PLAN.md — Config schema extension (boot_mode, restart policy) and ServiceManager trait
- [x] 04-02-PLAN.md — systemd implementation for Linux service registration
- [x] 04-03-PLAN.md — launchd implementation for macOS service registration
- [x] 04-04-PLAN.md — Install/uninstall CLI commands and status integration

### Phase 5: Interactive Setup Wizard
**Goal**: First-time users are guided through configuration with sensible defaults
**Depends on**: Phase 4
**Requirements**: INST-02, INST-03, INST-04, INST-05, CONF-01, CONF-02, CONF-03, CONF-05
**Success Criteria** (what must be TRUE):
  1. Running the CLI for first time launches interactive wizard
  2. Wizard prompts for username and password for basic auth
  3. Wizard prompts for port and hostname with sensible defaults shown
  4. User can skip API key configuration to set it later in opencode
  5. User can view current config via `opencode-cloud config`
  6. User can modify config values via `opencode-cloud config set <key> <value>`
  7. User can pass environment variables to opencode container
**Plans**: 3 plans

Plans:
- [x] 05-01-PLAN.md — Config schema extension (auth, env) and read-only config commands (show, get, reset)
- [x] 05-02-PLAN.md — Config mutation commands (set, env set/list/remove)
- [x] 05-03-PLAN.md — Interactive setup wizard with auto-trigger and setup command

### Phase 6: Security and Authentication
**Goal**: Service is secure by default with explicit opt-in for network exposure
**Depends on**: Phase 5
**Requirements**: SECU-01, SECU-02, SECU-03, SECU-04
**Note**: PAM-based authentication - opencode authenticates via PAM, so opencode-cloud manages system users in the container. Credentials are stored in the container's /etc/shadow (not in host config file). Users are tracked in config.users[] for persistence across rebuilds.
**Success Criteria** (what must be TRUE):
  1. Basic authentication is required to access the opencode web UI
  2. User manages container users via `occ user add/remove/list/passwd/enable/disable`
  3. Service binds to localhost (127.0.0.1) by default
  4. User must explicitly configure network exposure (0.0.0.0 binding)
  5. Warning is displayed when enabling network exposure
  6. Service works correctly behind AWS ALB/ELB with SSL termination (trust_proxy config)
**Plans**: 5 plans

Plans:
- [x] 06-01-PLAN.md — Config schema extension (bind_address, trust_proxy, rate_limit, users) and Docker exec/users modules
- [x] 06-02-PLAN.md — occ user commands (add, remove, list, passwd, enable, disable)
- [x] 06-03-PLAN.md — Network binding controls (bind_address config, container port binding, security warnings)
- [x] 06-04-PLAN.md — Status Security section and wizard PAM user creation integration
- [x] 06-05-PLAN.md — Config support for trust_proxy, rate_limit_*, allow_unauthenticated_network

### Phase 7: Update and Maintenance
**Goal**: User can update opencode and monitor service health
**Depends on**: Phase 6
**Requirements**: LIFE-06, LIFE-07, CONF-06
**Success Criteria** (what must be TRUE):
  1. User can update to latest opencode via `opencode-cloud update`
  2. Update preserves existing configuration and data volumes
  3. Health check endpoint available at `/health` for monitoring tools
  4. Configuration is validated on service startup with clear error messages for invalid config
**Plans**: 2 plans

Plans:
- [x] 07-01-PLAN.md — Update command with rollback support and progress feedback
- [x] 07-02-PLAN.md — Health check endpoint and config validation with actionable errors

### Phase 8: Polish and Documentation
**Goal**: CLI provides excellent UX with clear help and clean uninstall
**Depends on**: Phase 7
**Requirements**: INST-06, INST-07, INST-08
**Success Criteria** (what must be TRUE):
  1. All commands display helpful usage via `--help`
  2. Error messages are clear and include actionable guidance
  3. User can cleanly uninstall via `opencode-cloud uninstall`
  4. Uninstall removes service registration, config files, and optionally Docker volumes
**Plans**: 1 plan

Plans:
- [x] 08-01-PLAN.md — Uninstall UX improvements (confirmation prompt, remaining file paths display)

### Phase 9: Dockerfile Version Pinning
**Goal**: Pin explicit versions for tools installed from GitHub in the Dockerfile to improve security and reproducibility
**Depends on**: Phase 8
**Requirements**: None (enhancement)
**Note**: The custom Dockerfile installs many tools from GitHub releases without version pinning. This phase will review each tool and set explicit versions for security and repeatability.
**Success Criteria** (what must be TRUE):
  1. All GitHub-installed tools have explicit version tags (not :latest)
  2. Version pinning documented in Dockerfile comments
  3. Image builds are reproducible given same Dockerfile
  4. Security: Supply chain risk reduced by pinning to known-good versions
**Plans**: 2 plans

Plans:
- [x] 09-01-PLAN.md — Pin all versions in Dockerfile with inline documentation
- [x] 09-02-PLAN.md — Create update script and CI automation for weekly version checks

### Phase 10: Remote Administration via Cockpit
**Goal**: Integrate and expose remote administration of the Docker container via Cockpit running in the container
**Depends on**: Phase 9
**Requirements**: None (enhancement)
**Note**: Provides a web-based admin interface for managing the containerized environment, complementing the CLI for users who prefer GUI access. Requires switching from tini to systemd as container init.
**Success Criteria** (what must be TRUE):
  1. Cockpit is installed and running in the Docker container
  2. Cockpit web interface is accessible via a dedicated port
  3. Authentication is integrated with existing opencode-cloud credentials
  4. User can manage container services and view system status via Cockpit
**Plans**: 3 plans

Plans:
- [x] 10-01-PLAN.md — Transform Dockerfile from tini to systemd with Cockpit packages
- [x] 10-02-PLAN.md — Config schema (cockpit_port, cockpit_enabled) and container creation for systemd
- [x] 10-03-PLAN.md — CLI commands (occ cockpit) and status/start output updates

### Phase 11: Remote Host Management
**Goal**: Allow occ command to remotely install and interact with Docker containers running on different hosts via SSH tunnel
**Depends on**: Phase 10
**Requirements**: None (enhancement)
**Note**: Extends the CLI to manage opencode instances across multiple machines via SSH tunnels to remote Docker daemons. Uses system ssh command for compatibility with existing SSH infrastructure (keys, agents, jump hosts).
**Success Criteria** (what must be TRUE):
  1. User can add remote hosts via `occ host add <name> <hostname>`
  2. User can list and manage hosts with `occ host list/show/edit/remove`
  3. Secure connection via SSH tunnel (uses existing SSH keys/agent)
  4. All container commands support `--host` flag for remote operations
  5. Default host can be set, commands use it when `--host` not specified
**Plans**: 3 plans

Plans:
- [x] 11-01-PLAN.md — Core host module (schema, storage, SSH tunnel, error types)
- [x] 11-02-PLAN.md — Host CLI commands (add, remove, list, show, edit, test, default)
- [x] 11-03-PLAN.md — Command routing (--host flag, DockerClient remote, output prefixing)

### Phase 12: Web Desktop UI Investigation
**Goal**: Investigate integrating a secure web-exposed full custom browser desktop UI such as Friend OS, WDE, or similar
**Depends on**: Phase 11
**Requirements**: None (research/enhancement)
**Note**: Research phase to evaluate web desktop environments that could provide a full graphical interface to the containerized environment, accessible securely via browser.
**Success Criteria** (what must be TRUE):
  1. Candidate web desktop solutions evaluated (Friend OS, WDE, others)
  2. Security implications documented (auth, isolation, network exposure)
  3. Integration feasibility assessed with current Docker architecture
  4. Recommendation made: proceed with implementation or defer

**Plans**: TBD

Plans:
- [ ] 12-01: TBD (research and evaluation)

### Phase 13: Container Security Tools
**Goal**: Add security scanning and secrets management tools to the container image
**Depends on**: Phase 11 (Remote Host Management)
**Requirements**: None (enhancement)
**Note**: Deferred from initial Dockerfile to reduce image size and build time. Adds trivy, gitleaks, hadolint, age, sops, mkcert.
**Success Criteria** (what must be TRUE):
  1. trivy installed for vulnerability scanning
  2. gitleaks installed for secret detection
  3. hadolint installed for Dockerfile linting
  4. age and sops installed for secrets management
  5. mkcert installed for local TLS certificate generation
**Plans**: TBD

Plans:
- [ ] 13-01: TBD (security tools installation)

### Phase 14: Versioning and Release Automation
**Goal**: Automated CI/CD for Docker images with version tracking and mismatch detection in CLI
**Depends on**: Phase 11 (Remote Host Management)
**Requirements**: None (enhancement)
**Note**: Combines version tracking with CI/CD automation. GitHub Actions build and push versioned Docker images to GHCR. CLI detects when local image version differs from CLI version and prompts to pull/rebuild. Includes version bump workflows for releases.
**Success Criteria** (what must be TRUE):
  1. GitHub Action workflow builds and pushes Docker images to GHCR on release
  2. Multi-arch images (amd64, arm64) built via buildx
  3. Images tagged with version (e.g., `ghcr.io/prizz/opencode-cloud:1.0.8`) and `:latest`
  4. Docker images include version label (`org.opencode-cloud.version`)
  5. On `occ start`, CLI detects if image version differs from CLI version
  6. User is prompted to pull new image when version mismatch detected
  7. `occ status` shows image version alongside CLI version
  8. Workflow for version bumps with user input (major/minor/patch selection)
  9. Version bump updates all relevant files (Cargo.toml, package.json, etc.) and creates git tag
**Plans**: 3 plans

Plans:
- [x] 14-01-PLAN.md — GitHub Actions Docker build workflow (buildx multi-arch, GHCR push, version labels)
- [x] 14-02-PLAN.md — Version detection in CLI (image version check, mismatch prompt, status display)
- [x] 14-03-PLAN.md — Version bump workflow (workflow_dispatch, semver calculation, tag creation)

### Phase 15: Prebuilt Image Option
**Goal**: Give users the choice between pulling a prebuilt Docker image (fast, ~2 min) or building from source (customizable, 30-60 min)
**Depends on**: Phase 5 (Interactive Setup Wizard), Phase 14 (CI/CD infrastructure)
**Requirements**: None (enhancement)
**Note**: CI/CD for prebuilt images was completed in Phase 14. This phase adds user-facing choice, CLI flags, config options, and provenance tracking. Users who don't need customization can pull prebuilt; those who want transparency can build locally.
**Success Criteria** (what must be TRUE):
  1. On first run, user is prompted: "Pull prebuilt image (~2 min) or build from source (30-60 min)?"
  2. Clear explanation of trade-offs shown (prebuilt = faster, build = customizable/auditable)
  3. New config option `image_source` with values: `prebuilt`, `build` (default: prebuilt)
  4. New config option `update_check` with values: `always`, `once`, `never` (default: always)
  5. `occ start --pull-sandbox-image` forces pull of prebuilt image
  6. `occ start --cached-rebuild-sandbox-image` rebuilds with cache
  7. `occ start --full-rebuild-sandbox-image` rebuilds without cache
  8. `occ update` respects image_source config (pulls if prebuilt, builds if build)
  9. `occ status` shows image provenance (source and registry)
**Plans**: 3 plans

Plans:
- [x] 15-01-PLAN.md — Config schema extension (image_source, update_check) and image state module for provenance
- [x] 15-02-PLAN.md — Start command enhancement (renamed flags, pull-or-build logic, first-run prompt)
- [x] 15-03-PLAN.md — Wizard integration, update command respects config, status shows provenance

### Phase 16: Code Quality Audit
**Goal**: Improve code maintainability by reducing nesting, eliminating duplication, and applying consistent patterns
**Depends on**: Phase 8 (Polish and Documentation)
**Requirements**: None (maintenance/refactoring)
**Note**: Audit the entire codebase for deeply nested logic, duplicated code, and inconsistent patterns. Apply early returns, guard clauses, and extract helper functions where appropriate. This is a refactoring-only phase with no functional changes.
**Success Criteria** (what must be TRUE):
  1. No function exceeds 3 levels of nesting (excluding match arms)
  2. Duplicated logic extracted into shared helpers or traits
  3. Early returns and guard clauses used consistently
  4. All files pass `cargo clippy` with no warnings
  5. No regression in test coverage or functionality
**Plans**: 2 plans

Plans:
- [x] 16-01-PLAN.md — Extract duplicated format_docker_error into shared output/errors.rs module
- [x] 16-02-PLAN.md — Extract duplicated URL formatting (Cockpit URL, remote address) into output/urls.rs

### Phase 17: Custom Bind Mounts
**Goal**: Allow users to specify local filesystem directories to mount into the Docker container
**Depends on**: Phase 5 (Interactive Setup Wizard - for config UI)
**Requirements**: None (enhancement)
**Note**: Users often want to work with local project directories inside the container. This phase adds configuration and CLI options to specify bind mounts that map host paths to container paths.
**Success Criteria** (what must be TRUE):
  1. User can add bind mounts via `occ mount add /host/path:/container/path`
  2. User can add multiple mounts (array in config)
  3. Mounts are applied when container starts
  4. User can specify read-only mounts via `:ro` suffix
  5. Invalid paths (non-existent directories) are validated before container start
  6. `occ start --mount /path:/container/path` allows one-time mount without persisting to config
  7. `occ status` shows active bind mounts
**Plans**: 3 plans

Plans:
- [x] 17-01-PLAN.md — Core mount module (parsing, validation, error types) and config schema extension
- [x] 17-02-PLAN.md — CLI mount subcommand (add/remove/list) and start command flags (--mount, --no-mounts)
- [x] 17-03-PLAN.md — Container creation with bind mounts and status display of active mounts

### Phase 18: CLI Sync Strategy
**Goal**: Establish patterns to keep the Rust CLI and Node CLI in sync through automatic passthrough delegation
**Depends on**: Phase 8 (Polish and Documentation)
**Requirements**: None (architecture/maintenance)
**Note**: The Rust CLI is the single source of truth. The Node CLI becomes a thin wrapper that spawns the Rust binary and passes all arguments through transparently. This phase implements the passthrough, adds parity tests, and documents the process.
**Success Criteria** (what must be TRUE):
  1. Documented strategy for CLI parity (Rust is source of truth, Node delegates)
  2. Node CLI spawns Rust binary with stdio: inherit for transparent passthrough
  3. Test suite dynamically discovers commands from Rust CLI and verifies Node can invoke each
  4. CI fails if Node CLI cannot invoke a Rust CLI command
  5. Clear process for adding new commands (add to Rust only, no Node changes needed)
**Plans**: 3 plans

Plans:
- [x] 18-01-PLAN.md — Node CLI passthrough implementation (spawn wrapper, binary resolution, error handling)
- [x] 18-02-PLAN.md — Parity test suite with dynamic command discovery and CI integration
- [x] 18-03-PLAN.md — Documentation (CONTRIBUTING.md CLI architecture, command addition guide)

### Phase 19: CI/CD Automation
**Goal**: Automate Docker image builds/uploads and version management via GitHub Actions with user-triggered workflows
**Depends on**: Phase 15 (Prebuilt Image Option)
**Requirements**: None (enhancement)
**Note**: Builds on Phase 15's prebuilt image infrastructure. Adds automated CI/CD workflows for building and pushing Docker images to GHCR, plus interactive version bump workflows that prompt for version type (major/minor/patch) and create git tags.
**Success Criteria** (what must be TRUE):
  1. GitHub Action workflow builds and pushes Docker images to GHCR on release
  2. Multi-arch images (amd64, arm64) built via buildx
  3. Workflow for version bumps with user input (major/minor/patch selection)
  4. Automatic git tag creation after version bump
  5. Version bump updates all relevant files (Cargo.toml, package.json, etc.)
  6. Release workflow triggered by tag push or manual dispatch
  7. Images tagged with version and :latest
**Plans**: TBD

Plans:
- [ ] 19-01: TBD (Docker image build and push workflow)
- [ ] 19-02: TBD (Version bump workflow with user input)

### Phase 20: One-Click Cloud Deploy
**Goal**: Provide "Deploy to Cloud" buttons in README that let users spin up a fully-configured opencode-cloud instance on major cloud providers with minimal configuration
**Depends on**: Phase 15 (Prebuilt Image Option), Phase 19 (CI/CD Automation)
**Requirements**: None (enhancement)
**Note**: Uses cloud-init, Terraform templates, or provider-specific launch configurations (AWS CloudFormation, GCP Deployment Manager, Azure ARM templates) to provision a VM with Docker and opencode-cloud pre-installed. Users click a button, configure basics (region, instance size, SSH key), and get a running instance.
**Success Criteria** (what must be TRUE):
  1. README contains "Deploy to AWS" button that launches CloudFormation stack
  2. README contains "Deploy to GCP" button that launches Deployment Manager
  3. README contains "Deploy to Azure" button (optional, based on complexity)
  4. README contains "Deploy to DigitalOcean" button (1-click app or similar)
  5. Deployed instances have opencode-cloud running and accessible
  6. Users only need to provide: region, instance size, SSH key, and credentials
  7. Infrastructure templates are maintained in repository
  8. Documentation explains what resources are created and estimated costs
**Plans**: TBD

Plans:
- [ ] 20-01: TBD (cloud-init script and base configuration)
- [ ] 20-02: TBD (AWS CloudFormation template and deploy button)
- [ ] 20-03: TBD (GCP/Azure/DigitalOcean templates)

### Phase 21: Use opencode Fork with PAM Authentication
**Goal**: Switch from mainline opencode to the pRizz fork (https://github.com/pRizz/opencode) which implements proper PAM-based web authentication
**Depends on**: Phase 6 (Security and Authentication)
**Requirements**: None (enhancement)
**Note**: The mainline opencode uses basic auth stored in config. The pRizz fork integrates with PAM for web authentication, allowing users created via `occ user add` to authenticate directly to the opencode web UI using the same container system users that Cockpit uses.
**Success Criteria** (what must be TRUE):
  1. Dockerfile updated to install opencode from https://github.com/pRizz/opencode
  2. Users created via `occ user add` can log into opencode web UI
  3. Authentication is consistent between Cockpit and opencode (same PAM users)
  4. Legacy auth_username/auth_password config fields deprecated or removed
  5. Documentation updated to reflect PAM-based authentication flow
**Plans**: TBD

Plans:
- [ ] 21-01: TBD (Dockerfile update and PAM integration)

### Phase 22: Dedupe CLI Logic
**Goal**: Consolidate CLI logic so the Rust CLI is the single source of truth and the Node CLI delegates to it
**Depends on**: Phase 18 (CLI Sync Strategy)
**Requirements**: None (architecture/maintenance)
**Note**: Currently both CLIs have separate implementations. This phase makes the Rust CLI the authoritative implementation, with the Node CLI either wrapping the Rust binary or using NAPI bindings that delegate all logic to the Rust core.
**Success Criteria** (what must be TRUE):
  1. Rust CLI contains all command logic
  2. Node CLI delegates to Rust (via NAPI bindings or subprocess)
  3. No duplicated business logic between CLIs
  4. Both CLIs produce identical output for all commands
  5. Adding new commands only requires changes to Rust CLI
**Plans**: TBD

Plans:
- [ ] 22-01: TBD (audit current duplication, design consolidation strategy)
- [ ] 22-02: TBD (implement consolidation)

### Phase 23: Container Shell Access
**Goal**: Provide quick terminal access to the running container via `occ shell` command
**Depends on**: Phase 3 (Service Lifecycle Commands)
**Requirements**: None (enhancement)
**Note**: Users often need quick shell access without going through Cockpit. This command provides `docker exec -it` functionality with a simpler interface.
**Success Criteria** (what must be TRUE):
  1. `occ shell` opens interactive bash session in running container
  2. `occ shell <command>` executes single command and returns output
  3. Works with remote hosts via `--host` flag
  4. Proper TTY handling for interactive sessions
  5. Exit code propagation from container commands
**Plans**: TBD

Plans:
- [ ] 23-01: TBD (shell command implementation)

### Phase 24: IDE Integration
**Goal**: Create extensions for popular IDEs to connect to and work with opencode containers
**Depends on**: Phase 11 (Remote Host Management)
**Requirements**: None (enhancement)
**Note**: IDE integration allows developers to use their preferred editor while leveraging the sandboxed container environment. Focus on VS Code first (largest market share), then JetBrains.
**Success Criteria** (what must be TRUE):
  1. VS Code extension can list running opencode containers
  2. VS Code extension can connect to container for remote development
  3. Extension respects existing SSH config for remote hosts
  4. File editing, terminal, and debugging work through extension
  5. Extension available on VS Code marketplace
**Plans**: TBD

Plans:
- [ ] 24-01: TBD (VS Code extension)
- [ ] 24-02: TBD (JetBrains plugin - optional)

### Phase 25: Container Templates
**Goal**: Provide pre-configured environment templates optimized for different development stacks
**Depends on**: Phase 2 (Docker Integration)
**Requirements**: None (enhancement)
**Note**: Different projects need different tools. Templates provide optimized Dockerfiles for specific stacks (Python ML with CUDA, Node.js with common tools, Rust development, etc.) without bloating the default image.
**Success Criteria** (what must be TRUE):
  1. `occ start --template python-ml` uses Python/ML-optimized image
  2. At least 3 templates available: python-ml, nodejs, rust
  3. Templates can be listed via `occ templates list`
  4. Custom templates can be added via config
  5. Template selection persisted in config for container
**Plans**: TBD

Plans:
- [ ] 25-01: TBD (template system architecture)
- [ ] 25-02: TBD (built-in templates)

### Phase 26: Secrets Management
**Goal**: Securely inject API keys and credentials into the container without exposing them in config files
**Depends on**: Phase 5 (Interactive Setup Wizard)
**Requirements**: None (enhancement)
**Note**: Users need to provide API keys (OpenAI, Anthropic, etc.) to opencode. Currently these may end up in config files or shell history. This phase provides secure secret injection using Docker secrets or environment variable management.
**Success Criteria** (what must be TRUE):
  1. `occ secret set OPENAI_API_KEY` securely stores secret
  2. Secrets injected into container as environment variables
  3. Secrets never written to plain-text config files
  4. `occ secret list` shows secret names (not values)
  5. `occ secret remove <name>` removes a secret
  6. Secrets persist across container restarts
**Plans**: TBD

Plans:
- [ ] 26-01: TBD (secure secret storage)
- [ ] 26-02: TBD (CLI commands and container injection)

### Phase 27: Windows Support
**Goal**: Full Windows compatibility for the CLI and Docker integration
**Depends on**: Phase 4 (Platform Service Installation)
**Requirements**: None (enhancement)
**Note**: Windows support was deferred from v1. This phase adds Windows service integration, path handling, and testing. Requires Docker Desktop or WSL2 with Docker.
**Success Criteria** (what must be TRUE):
  1. CLI compiles and runs on Windows
  2. Windows service registration (similar to systemd/launchd)
  3. Config paths follow Windows conventions (%APPDATA%)
  4. All existing commands work on Windows
  5. CI includes Windows build and test
**Plans**: TBD

Plans:
- [ ] 27-01: TBD (Windows compilation and path handling)
- [ ] 27-02: TBD (Windows service integration)
- [ ] 27-03: TBD (CI and testing)

### Phase 28: Remote Host Setup Wizard
**Goal**: Run the interactive setup wizard for remote hosts via `occ setup --host <name>`
**Depends on**: Phase 11 (Remote Host Management), Phase 5 (Interactive Setup Wizard)
**Requirements**: None (enhancement)
**Note**: Currently setup wizard only works locally. This phase extends it to configure remote hosts, creating users in remote containers and managing remote config. Builds on the --host flag added to setup in Phase 15.
**Success Criteria** (what must be TRUE):
  1. `occ setup --host <name>` runs wizard targeting remote host
  2. User creation happens in remote container
  3. Config changes apply to remote host's container
  4. Restart prompt respects remote host context
  5. Clear messaging indicates remote vs local operation
**Plans**: TBD

Plans:
- [ ] 28-01: TBD (remote wizard implementation)

### Phase 29: DockerHub README Elaboration
**Goal**: Elaborate and improve the opencode-cloud-sandbox README.dockerhub.md content for better discoverability and user onboarding
**Depends on**: Phase 14 (Versioning and Release Automation - for GHCR/DockerHub publishing)
**Requirements**: None (documentation)
**Note**: The Docker image is published to registries but the README.dockerhub.md may need more detailed content including usage examples, configuration options, environment variables, volume mounts, and integration guides to help users get started quickly.
**Success Criteria** (what must be TRUE):
  1. README.dockerhub.md contains clear getting started instructions
  2. All environment variables are documented with examples
  3. Volume mount paths are explained
  4. Port mappings are documented
  5. Example docker-compose.yml included
  6. Links to full documentation and CLI tool
**Plans**: TBD

Plans:
- [ ] 29-01: TBD (README.dockerhub.md content elaboration)

### Phase 30: Deep Nesting Refactor
**Goal**: Audit codebase for deeply nested code and refactor using early returns, guard clauses, let-else patterns, and helper functions to improve readability and maintainability
**Depends on**: Phase 16 (Code Quality Audit)
**Requirements**: None (maintenance/refactoring)
**Note**: Systematic review of all Rust code for functions with excessive nesting depth. Apply consistent patterns: early returns for error conditions, let-else for Option/Result handling, extract complex closures to named functions. This builds on Phase 16's foundation.
**Success Criteria** (what must be TRUE):
  1. No function exceeds 3 levels of nesting (excluding match arms)
  2. Guard clauses used for all precondition checks
  3. let-else pattern used where applicable for Option/Result unwrapping
  4. Complex closures extracted to named helper functions
  5. All files pass `cargo clippy` with no warnings
  6. No regression in test coverage or functionality
**Plans**: TBD

Plans:
- [ ] 30-01: TBD (audit and refactor)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> ... -> 22 -> 23 -> 24 -> 25 -> 26 -> 27

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Project Foundation | 2/2 | ✓ Complete | 2026-01-18 |
| 2. Docker Integration | 3/3 | ✓ Complete | 2026-01-19 |
| 3. Service Lifecycle Commands | 2/2 | ✓ Complete | 2026-01-19 |
| 4. Platform Service Installation | 4/4 | ✓ Complete | 2026-01-19 |
| 5. Interactive Setup Wizard | 3/3 | ✓ Complete | 2026-01-20 |
| 6. Security and Authentication | 5/5 | ✓ Complete | 2026-01-20 |
| 7. Update and Maintenance | 2/2 | ✓ Complete | 2026-01-22 |
| 8. Polish and Documentation | 1/1 | ✓ Complete | 2026-01-22 |
| 9. Dockerfile Version Pinning | 2/2 | ✓ Complete | 2026-01-22 |
| 10. Remote Administration via Cockpit | 3/3 | ✓ Complete | 2026-01-22 |
| 11. Remote Host Management | 3/3 | ✓ Complete | 2026-01-23 |
| 12. Web Desktop UI Investigation | - | Deferred | - |
| 13. Container Security Tools | - | Deferred | - |
| 14. Versioning and Release Automation | 3/3 | ✓ Complete | 2026-01-23 |
| 15. Prebuilt Image Option | 3/3 | ✓ Complete | 2026-01-24 |
| 16. Code Quality Audit | 2/2 | ✓ Complete | 2026-01-25 |
| 17. Custom Bind Mounts | 3/3 | ✓ Complete | 2026-01-25 |
| 18. CLI Sync Strategy | 3/3 | ✓ Complete | 2026-01-25 |
| 19. CI/CD Automation | - | MERGED | - |
| 20. One-Click Cloud Deploy | 0/3 | Not started | - |
| 21. Use opencode Fork with PAM Auth | 0/1 | Not started | - |
| 22. Dedupe CLI Logic | 0/2 | Not started | - |
| 23. Container Shell Access | 0/1 | Not started | - |
| 24. IDE Integration | 0/2 | Not started | - |
| 25. Container Templates | 0/2 | Not started | - |
| 26. Secrets Management | 0/2 | Not started | - |
| 27. Windows Support | 0/3 | Not started | - |
| 28. Remote Host Setup Wizard | 0/1 | Not started | - |
| 29. DockerHub README Elaboration | 0/1 | Not started | - |
| 30. Deep Nesting Refactor | 0/1 | Not started | - |

---
*Roadmap created: 2026-01-18*
*Last updated: 2026-01-25 (Phase 18 complete)*
