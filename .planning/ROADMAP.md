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
- [ ] **Phase 5: Interactive Setup Wizard** - Guided first-run experience
- [ ] **Phase 6: Security and Authentication** - Basic auth, localhost binding, network exposure opt-in
- [ ] **Phase 7: Update and Maintenance** - Update command, health check endpoint
- [ ] **Phase 8: Polish and Documentation** - Help docs, error messages, uninstall cleanup
- [ ] **Phase 9: Dockerfile Version Pinning** - Pin explicit versions for GitHub-installed tools
- [ ] **Phase 10: Remote Administration via Cockpit** - Integrate and expose remote admin of Docker container via Cockpit
- [ ] **Phase 11: Remote Host Management** - Allow occ to remotely install and interact with Docker containers on different hosts
- [ ] **Phase 12: Web Desktop UI Investigation** - Investigate integrating secure web-exposed desktop UI (Friend OS, WDE, etc.)
- [ ] **Phase 13: Container Security Tools** - Add trivy, gitleaks, hadolint, age, sops, mkcert to container

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
**Plans**: TBD

Plans:
- [ ] 05-01: TBD (interactive wizard flow)
- [ ] 05-02: TBD (config view/edit commands)

### Phase 6: Security and Authentication
**Goal**: Service is secure by default with explicit opt-in for network exposure
**Depends on**: Phase 5
**Requirements**: SECU-01, SECU-02, SECU-03, SECU-04
**Note**: Auth credentials configured here are for opencode's web UI (passed to opencode process). Credentials are stored in the host config file at the platform-appropriate path. Future remote terminal/desktop features (v2/v3) may introduce a separate auth layer.
**Success Criteria** (what must be TRUE):
  1. Basic authentication is required to access the opencode web UI
  2. User chooses auth method during wizard; credentials passed to opencode process
  3. Service binds to localhost (127.0.0.1) by default
  4. User must explicitly configure network exposure (0.0.0.0 binding)
  5. Warning is displayed when enabling network exposure
  6. Service works correctly behind AWS ALB/ELB with SSL termination
**Plans**: TBD

Plans:
- [ ] 06-01: TBD (opencode authentication configuration)
- [ ] 06-02: TBD (network binding controls)

### Phase 7: Update and Maintenance
**Goal**: User can update opencode and monitor service health
**Depends on**: Phase 6
**Requirements**: LIFE-06, LIFE-07, CONF-06
**Success Criteria** (what must be TRUE):
  1. User can update to latest opencode via `opencode-cloud update`
  2. Update preserves existing configuration and data volumes
  3. Health check endpoint available at `/health` for monitoring tools
  4. Configuration is validated on service startup with clear error messages for invalid config
**Plans**: TBD

Plans:
- [ ] 07-01: TBD (update command)
- [ ] 07-02: TBD (health check and config validation)

### Phase 8: Polish and Documentation
**Goal**: CLI provides excellent UX with clear help and clean uninstall
**Depends on**: Phase 7
**Requirements**: INST-06, INST-07, INST-08
**Success Criteria** (what must be TRUE):
  1. All commands display helpful usage via `--help`
  2. Error messages are clear and include actionable guidance
  3. User can cleanly uninstall via `opencode-cloud uninstall`
  4. Uninstall removes service registration, config files, and optionally Docker volumes
**Plans**: TBD

Plans:
- [ ] 08-01: TBD (help documentation)
- [ ] 08-02: TBD (error handling and uninstall)

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
**Plans**: TBD

Plans:
- [ ] 09-01: TBD (audit and pin versions)

### Phase 10: Remote Administration via Cockpit
**Goal**: Integrate and expose remote administration of the Docker container via Cockpit running in the container
**Depends on**: Phase 9
**Requirements**: None (enhancement)
**Note**: Provides a web-based admin interface for managing the containerized environment, complementing the CLI for users who prefer GUI access.
**Success Criteria** (what must be TRUE):
  1. Cockpit is installed and running in the Docker container
  2. Cockpit web interface is accessible via a dedicated port
  3. Authentication is integrated with existing opencode-cloud credentials
  4. User can manage container services and view system status via Cockpit
**Plans**: TBD

Plans:
- [ ] 10-01: TBD (Cockpit installation and configuration)

### Phase 11: Remote Host Management
**Goal**: Allow occ command to remotely install and interact with Docker containers running on different hosts
**Depends on**: Phase 10
**Requirements**: None (enhancement)
**Note**: Extends the CLI to manage opencode instances across multiple machines via SSH or Docker API, enabling centralized management of distributed deployments.
**Success Criteria** (what must be TRUE):
  1. User can add remote hosts via `occ host add <hostname>`
  2. User can list and manage containers on remote hosts
  3. Secure connection to remote Docker daemons (SSH tunnel or TLS)
  4. Commands work transparently across local and remote hosts
**Plans**: TBD

Plans:
- [ ] 11-01: TBD (remote host management)

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
**Depends on**: Phase 12
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

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9 -> 10 -> 11 -> 12 -> 13

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Project Foundation | 2/2 | ✓ Complete | 2026-01-18 |
| 2. Docker Integration | 3/3 | ✓ Complete | 2026-01-19 |
| 3. Service Lifecycle Commands | 2/2 | ✓ Complete | 2026-01-19 |
| 4. Platform Service Installation | 4/4 | ✓ Complete | 2026-01-19 |
| 5. Interactive Setup Wizard | 0/2 | Not started | - |
| 6. Security and Authentication | 0/2 | Not started | - |
| 7. Update and Maintenance | 0/2 | Not started | - |
| 8. Polish and Documentation | 0/2 | Not started | - |
| 9. Dockerfile Version Pinning | 0/1 | Not started | - |
| 10. Remote Administration via Cockpit | 0/1 | Not started | - |
| 11. Remote Host Management | 0/1 | Not started | - |
| 12. Web Desktop UI Investigation | 0/1 | Not started | - |
| 13. Container Security Tools | 0/1 | Not started | - |

---
*Roadmap created: 2026-01-18*
*Last updated: 2026-01-19 (Phase 13 added - security tools deferred)*
