# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-18)

**Core value:** Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.
**Current focus:** Phase 21 complete — Use opencode Fork with PAM Authentication

## Current Position

Phase: 21 of 21 (Use opencode Fork with PAM Authentication)
Plan: 2 of 2 in current phase
Status: Phase complete
**Next Phase:** 32 (Make automated (or manually triggered) scripts/github actions in CI for generating and uploading an AWS AMI so that cloud formation/installation can be streamlined and quickened)
Last activity: 2026-01-26 - Executed 21-01, 21-02 (Dockerfile fork installation, broker, PAM config, documentation)

Progress: [##############################] 100%

Note: Phases 12 (Web Desktop UI) and 13 (Container Security Tools) are DEFERRED. Phase 19 (CI/CD Automation) merged into Phase 14. Phase 22 (Dedupe CLI Logic) merged into Phase 18.

## Performance Metrics

**Velocity:**
- Total plans completed: 48
- Average duration: 5 min
- Total execution time: ~4.5 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 2 | 14 min | 7 min |
| 02 | 3 | 18 min | 6 min |
| 03 | 2 | 16 min | 8 min |
| 04 | 4 | 21 min | 5 min |
| 05 | 3 | 19 min | 6 min |
| 06 | 5 | 33 min | 7 min |
| 07 | 2 | 11 min | 6 min |
| 08 | 1 | 2 min | 2 min |
| 09 | 2 | 13 min | 7 min |
| 10 | 3 | 15 min | 5 min |
| 11 | 3 | 19 min | 6 min |
| 14 | 3 | 15 min | 5 min |
| 15 | 3 | 19 min | 6 min |
| 16 | 2 | 12 min | 6 min |
| 17 | 3 | 11 min | 4 min |
| 18 | 6 | 25 min | 4 min |

**Recent Trend:**
- Last 5 plans: 2 min, 3 min, 3 min, 5 min, 3 min
- Trend: Stable (~3 min average)

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Init]: Use opencode's built-in basic auth (avoid reinventing)
- [Init]: Google Drive sync deferred to v2 (reduce scope)
- [Phase 1]: Project renamed to `opencode-cloud` (CLI: `occ`)
- [Phase 1]: Rust core + NAPI-RS bindings architecture
- [Phase 1]: Monorepo: packages/core, packages/cli-rust, packages/cli-node
- [Phase 1]: JSONC config at ~/.config/opencode-cloud/config.json
- [Phase 1]: Windows support deferred to v2
- [01-01]: Feature flag for NAPI: Use cargo feature 'napi' to conditionally compile NAPI bindings
- [01-01]: Manual JS bindings: Generate index.js/index.d.ts manually due to napi-cli type generation issues
- [01-01]: Dual crate-type: Core library uses both cdylib (for NAPI) and rlib (for Rust CLI)
- [01-02]: XDG paths for macOS: Use ~/.config/ instead of ~/Library for consistency
- [01-02]: Singleton via kill -0: Use subprocess call for process existence check
- [01-02]: Strict config validation: deny_unknown_fields rejects unknown keys
- [Phase 1]: No prebuilt binaries: Both npm and crates.io compile from source; Rust 1.85+ required for transparency
- [02-01]: Bollard 0.18 with buildkit feature for Docker operations
- [02-01]: Connection errors categorized: NotRunning, PermissionDenied, Connection, Timeout
- [02-01]: Dockerfile embedded via include_str! for single-binary distribution
- [02-01]: Ubuntu 24.04 LTS base image for stability
- [02-02]: Manual retry loop instead of tokio-retry due to async closure capture limitations
- [02-02]: Per-layer progress bars for downloads, spinners for build steps
- [02-02]: Exponential backoff: 1s, 2s, 4s (max 3 attempts) for image pull
- [02-03]: Named volumes over bind mounts for cross-platform compatibility
- [02-03]: Port binding to 127.0.0.1 only for security (prevents external access)
- [02-03]: Volume label 'managed-by: opencode-cloud' for identification
- [03-01]: DockerClient::new() is synchronous; verify_connection() validates async
- [03-01]: Port availability pre-checked before container creation
- [03-01]: Quiet mode (-q) outputs only URL for scripting
- [03-02]: Status quiet mode exits 0/1 for running/stopped (no output)
- [03-02]: Logs follow mode default (--no-follow for one-shot)
- [03-02]: chrono used for timestamp parsing (minimal features: std, clock)
- [04-01]: ServiceManager trait uses Result<T> return types for all operations
- [04-01]: Platform detection via cfg!(target_os) compile-time macros
- [04-02]: User mode directory ~/.config/systemd/user/ for no-root services
- [04-02]: Quote executable paths with spaces in ExecStart/ExecStop
- [04-02]: Rate limiting formula: StartLimitIntervalSec = delay * retries * 2
- [04-03]: plist crate for XML serialization instead of manual templating
- [04-03]: Modern launchctl bootstrap/bootout syntax over deprecated load/unload
- [04-03]: User mode by default (~/Library/LaunchAgents/) for non-root installation
- [04-04]: dialoguer for interactive confirmation prompts
- [04-04]: Idempotent uninstall: exit 0 if service not installed
- [04-04]: --volumes requires --force for safety
- [05-01]: Password masking: auth_password always shown as ******** in output
- [05-01]: Default subcommand: occ config defaults to show using optional subcommand pattern
- [05-01]: Key aliases: config get supports both short (port) and full (opencode_web_port) names
- [05-01]: comfy-table for CLI table output formatting
- [05-02]: Password via CLI rejected: Security error with instructions for interactive prompt
- [05-02]: Username validation: 3-32 chars, alphanumeric and underscore only
- [05-02]: Env var update: Remove existing entry with same key before adding new
- [05-03]: WizardState struct collects values before applying to Config
- [05-03]: Quick setup mode: single confirmation skips port/hostname prompts
- [05-03]: Random password: 24-char alphanumeric using rand crate
- [05-03]: Auto-trigger excludes setup and config commands
- [06-01]: Password via stdin: chpasswd reads username:password from stdin for security
- [06-01]: Default bind_address: 127.0.0.1 (localhost-only by default)
- [06-01]: validate_bind_address accepts localhost, IPv4, IPv6, and bracketed [::1]
- [06-01]: list_users filters by /home/ directory to exclude system users
- [06-02]: Container running check: All user commands require container to be running first
- [06-02]: Config persistence: User additions/removals update config.users array
- [06-02]: Last user protection: Cannot remove last user without --force
- [06-02]: Network security warnings: start command warns when exposed but no users
- [06-05]: Double confirmation for allow_unauthenticated_network: Two Y/N prompts required
- [06-05]: Warning at >100 rate_limit_attempts for security awareness
- [06-05]: Warning at <10s rate_limit_window for false positive awareness
- [06-04]: Security section displays in status even when container stopped (config relevant for restart)
- [06-04]: Legacy auth_username/auth_password cleared to empty strings (not None) for schema compatibility
- [06-04]: First-start check uses container_exists (container is the runtime boundary)
- [06-04]: Existing containers can restart without users (migration safety for existing deployments)
- [07-01]: Tag current image as "previous" before update: Enables rollback to last known-good version
- [07-01]: Passwords NOT preserved during update: Config only stores usernames for security
- [07-01]: Step-by-step progress with confirmation: 5 steps for update, 4 for rollback
- [07-01]: Use bollard TagImageOptions: Modern API instead of deprecated struct
- [07-02]: Health check uses 5-second timeout for quick failure detection
- [07-02]: Validation stops at first error, returns all warnings
- [07-02]: ValidationError includes field, message, and fix_command
- [07-02]: Health states: Healthy (green), Service starting (yellow), Unhealthy (red), Check failed (yellow)
- [08-01]: Default confirmation to 'n' for uninstall safety
- [08-01]: Show actual resolved paths using get_config_dir/get_data_dir
- [09-01]: APT wildcards allow patch updates: Use pkg=X.Y.* pattern for security patches
- [09-01]: Security exceptions: ca-certificates, gnupg, openssh-client marked UNPINNED
- [09-01]: Self-managing installers trusted: mise, rustup, starship, oh-my-zsh, uv, opencode
- [09-01]: Go runtime pinned to minor: go@1.24 instead of @latest
- [09-02]: POSIX-compatible patterns: Use sed with | delimiter and grep -- for cross-platform compatibility
- [09-02]: Weekly schedule: Monday 9am UTC for version checks via CI workflow
- [10-01]: systemd as PID 1: Required for Cockpit socket activation
- [10-01]: Minimal Cockpit packages: cockpit-ws, cockpit-system, cockpit-bridge
- [10-01]: AllowUnencrypted=true: TLS terminated externally like opencode
- [10-01]: Keep tini in image: Backward compatibility, though systemd now default
- [10-01]: STOPSIGNAL SIGRTMIN+3: Proper systemd shutdown signal
- [10-02]: Default cockpit_port 9090: Standard Cockpit port
- [10-02]: Default cockpit_enabled true: Cockpit integrated in image
- [10-02]: CAP_SYS_ADMIN capability: Required for systemd cgroup operations
- [10-02]: tmpfs for /run and /tmp: Required for systemd runtime
- [10-02]: cgroup mount read-only: Security - prevent container cgroup manipulation
- [10-03]: Check cockpit_enabled before container status: Better error ordering for user feedback
- [10-03]: Browser address normalization: Use 127.0.0.1 for 0.0.0.0/:: bind addresses
- [11-01]: SSH tunnel via system ssh command (not library) - uses existing SSH config/agent
- [11-01]: BatchMode=yes for non-interactive auth - fail fast if key not in agent
- [11-01]: StrictHostKeyChecking=accept-new - accept new hosts automatically on first connection
- [11-01]: Port 0 binding to find available local port for tunnel
- [11-01]: Drop trait ensures SSH process cleanup on tunnel destruction
- [11-02]: Connection verification by default on add with --no-verify escape hatch
- [11-02]: Confirmation prompt on remove with --force bypass
- [11-02]: Quiet mode on test exits 0/1 for scripting
- [11-02]: Names-only mode on list for shell loops
- [11-02]: JSON output on show for programmatic consumption
- [11-02]: Partial updates on edit - only specified fields change
- [11-03]: Global --host flag on Cli struct for all commands to inherit
- [11-03]: resolve_docker_client helper centralizes host resolution logic
- [11-03]: Host resolution order: explicit --host > default_host from hosts.json > local Docker
- [11-03]: format_host_message helper ensures consistent [hostname] prefix formatting
- [11-03]: User subcommands accept DockerClient reference instead of creating own
- [14-01]: GHCR as primary Docker registry: ghcr.io/prizz/opencode-cloud
- [14-01]: Version label: org.opencode-cloud.version in Dockerfile
- [14-01]: Multi-arch via QEMU: Simpler than ARM runners, acceptable for slow image builds
- [14-01]: VERSION file at /etc/opencode-cloud-version for runtime access
- [14-02]: Version check skipped during rebuilds: No point checking if we're rebuilding anyway
- [14-02]: Dev version always compatible: Local builds treated as compatible
- [14-02]: Prompt offers rebuild (not pull): Pull option deferred to Phase 15 (Prebuilt Image)
- [14-03]: Two-tag strategy: Both v* and release/v* for backward compatibility
- [14-03]: GitHub Actions bot for commits: Clear audit trail, avoids CI loops
- [15-01]: Image source defaults to "prebuilt": Most users want fast setup; pull from GHCR is faster than local build
- [15-01]: Update check defaults to "always": Security patches should be discovered automatically
- [15-01]: State file location in data directory: Image state is operational/ephemeral (which image is current)
- [15-01]: ISO8601 timestamps via chrono::Utc: Standardized format, timezone-aware, sortable
- [15-03]: Wizard always prompts for image source: Even in quick setup, choice between 2-min pull vs 60-min build warrants explicit user input
- [15-03]: Update shows informational messages: Display which method (build/pull) is used and how to change it
- [15-03]: Graceful provenance display: Status shows image provenance when state file exists, silently skips if missing
- [16-01]: Docker error formatting centralized: output/errors.rs module with format_docker_error, show_docker_error
- [16-02]: URL helper location: output/urls.rs module consistent with output module pattern
- [17-01]: Mount format: /host:/container[:ro|rw] with rw as default
- [17-01]: System paths warning: /etc, /usr, /bin, /sbin, /lib, /var
- [17-01]: Validation uses canonicalize() to resolve symlinks and verify existence
- [17-02]: Mount group follows user/ pattern: Subcommand group with MountArgs, MountCommands enum
- [17-02]: Duplicate detection by host path: Adding same host path twice is idempotent, not an error
- [17-02]: Restart note in output: CLI reminds users to restart container for changes to take effect
- [17-03]: Restart/update commands pass None for bind_mounts: User can restart with mounts after if needed
- [17-03]: Status displays bind mounts with source indicator (config vs cli)
- [18-01]: stdio: inherit for passthrough: Preserves TTY detection, colors, interactive prompts
- [18-01]: Git hook excludes cli-node README: Allow package-specific wrapper documentation
- [18-04]: optionalDependencies pattern: Follow esbuild/swc model for platform-specific binary distribution
- [18-04]: Six platform packages: darwin-arm64, darwin-x64, linux-x64, linux-arm64, linux-x64-musl, linux-arm64-musl

### Pending Todos

1. **Make README sync more robust** (tooling) - Add CI check to verify package READMEs match root; consider husky/lefthook for cross-platform hook management.
2. **Handle pnpm v10 blocked install scripts** (tooling) - Add runtime guard with actionable error for pnpm users; document workaround in README.
3. **Optimize Docker build with BuildKit caches** (tooling) - Revisit safe cache mounts to speed up builds without permission issues.
4. **Sustain opencode nginx routing** (tooling) - Replace hardcoded proxy route lists with a more robust strategy.
5. **Investigate hanging graceful stop** (tooling) - Diagnose missing stop progress when rebuilding a running container.
6. **Fix missing export step display** (tooling) - Ensure "exporting to image" appears as the final Docker build step.
7. **Use occ in CloudFormation scripts** (tooling) - Replace manual bootstrap wiring with occ-driven setup.

### Roadmap Evolution

- Phase 9 added: Dockerfile Version Pinning (pin explicit versions for GitHub-installed tools)
- Phase 10 added: Remote Administration via Cockpit (integrate and expose remote admin of Docker container)
- Phase 11 added: Remote Host Management (occ manages containers on different hosts)
- Phase 12 DEFERRED: Web Desktop UI Investigation (Friend OS, WDE, etc.)
- Phase 13 DEFERRED: Container Security Tools (trivy, gitleaks, hadolint, etc.)
- Phase 14 expanded: Versioning and Release Automation (merged with Phase 19 CI/CD Automation)
- Phase 15: Prebuilt Image Option (pull vs build from source)
- Phase 16: Code Quality Audit (reduce nesting, eliminate duplication)
- Phase 17: Custom Bind Mounts (mount local directories into container)
- Phase 18 EXPANDED: CLI Sync Strategy + Prebuilt Binary Distribution (merged Phase 22)
- Phase 19 MERGED: CI/CD Automation merged into Phase 14
- Phase 20: One-Click Cloud Deploy (deploy buttons for AWS, GCP, Azure, DigitalOcean) — deferred in favor of Phase 21
- Phase 21: Use opencode Fork with PAM Authentication (switch to pRizz/opencode fork) — RE-PRIORITIZED: next phase to work on
- Phase 22 MERGED: Dedupe CLI Logic merged into Phase 18
- Phase 23: Container Shell Access (`occ shell` for quick terminal access)
- Phase 24: IDE Integration (VS Code and JetBrains extensions)
- Phase 25: Container Templates (pre-configured stacks: Python ML, Node.js, Rust)
- Phase 26: Secrets Management (secure API key injection)
- Phase 27: Windows Support (full Windows compatibility)
- Phase 28: Remote Host Setup Wizard (run setup wizard for remote hosts)
- Phase 29 added: DockerHub README Elaboration (improve README.dockerhub.md for discoverability)
- Phase 30 added: Deep Nesting Refactor (early returns, guard clauses, let-else, helper functions)
- Phase 31 added: Update rust and node cli READMEs with latest project features
- Phase 32 added: Make automated (or manually triggered) scripts/github actions in CI for generating and uploading an AWS AMI so that cloud formation/installation can be streamlined and quickened

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-01-26
Stopped at: Phase 21 complete (21-01, 21-02 executed)
Resume file: None
Next step: /gsd:verify-work 21 (UAT) to test PAM authentication; or proceed to Phase 20 (One-Click Cloud Deploy)
