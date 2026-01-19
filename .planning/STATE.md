# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-18)

**Core value:** Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.
**Current focus:** Phase 4 - Platform Service Installation (In Progress)

## Current Position

Phase: 4 of 12 (Platform Service Installation)
Plan: 3 of 4 in current phase
Status: In progress
Last activity: 2026-01-19 - Completed 04-03-PLAN.md (macOS launchd Service Manager)

Progress: [#########.] 50%

## Performance Metrics

**Velocity:**
- Total plans completed: 10
- Average duration: 7 min
- Total execution time: 1.1 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 2 | 14 min | 7 min |
| 02 | 3 | 18 min | 6 min |
| 03 | 2 | 16 min | 8 min |
| 04 | 3 | 18 min | 6 min |

**Recent Trend:**
- Last 5 plans: 4 min, 6 min, 4 min, 8 min
- Trend: Stable (~6 min average)

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

### Pending Todos

1. **Make README sync more robust** (tooling) - Add CI check to verify package READMEs match root; consider husky/lefthook for cross-platform hook management.
2. **Handle pnpm v10 blocked install scripts** (tooling) - Add runtime guard with actionable error for pnpm users; document workaround in README.

### Roadmap Evolution

- Phase 9 added: Dockerfile Version Pinning (pin explicit versions for GitHub-installed tools)
- Phase 10 added: Remote Administration via Cockpit (integrate and expose remote admin of Docker container)
- Phase 11 added: Remote Host Management (occ manages containers on different hosts)
- Phase 12 added: Web Desktop UI Investigation (Friend OS, WDE, etc.)

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-01-19 19:58:00 UTC
Stopped at: Completed 04-03-PLAN.md (macOS launchd Service Manager)
Resume file: None
