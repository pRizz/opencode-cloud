---
phase: 03-service-lifecycle-commands
plan: 01
subsystem: cli
tags: [clap, indicatif, bollard, docker, commands, spinner]

# Dependency graph
requires:
  - phase: 02-03
    provides: Docker container lifecycle operations (setup_and_start, stop_service)
provides:
  - Start command with port check, auto-build, spinner feedback
  - Stop command with 30s graceful timeout
  - Restart command (stop + start with single spinner)
  - CommandSpinner with elapsed time and quiet mode support
affects: [03-status-logs-commands, 04-tunnel-integration]

# Tech tracking
tech-stack:
  added: [webbrowser, humantime]
  patterns:
    - "CommandSpinner for async operation feedback with elapsed time"
    - "Idempotent command behavior (no error when already in target state)"
    - "Actionable Docker error messages with troubleshooting links"

key-files:
  created:
    - packages/cli-rust/src/output/mod.rs
    - packages/cli-rust/src/output/spinner.rs
    - packages/cli-rust/src/commands/mod.rs
    - packages/cli-rust/src/commands/start.rs
    - packages/cli-rust/src/commands/stop.rs
    - packages/cli-rust/src/commands/restart.rs
  modified:
    - packages/cli-rust/src/lib.rs
    - packages/cli-rust/Cargo.toml
    - Cargo.toml

key-decisions:
  - "DockerClient::new() is synchronous; verify_connection() validates async"
  - "Port availability pre-checked before container creation"
  - "Quiet mode (-q) outputs only URL for scripting"

patterns-established:
  - "Output module for CLI terminal utilities (spinner, future: colors)"
  - "Commands module for service lifecycle operations"
  - "Async command execution via tokio::runtime::Runtime in sync CLI"

# Metrics
duration: 12min
completed: 2026-01-19
---

# Phase 3 Plan 1: Start, Stop, Restart Commands Summary

**Three service lifecycle commands (start/stop/restart) with spinner feedback, port conflict detection, and idempotent behavior**

## Performance

- **Duration:** 12 min
- **Started:** 2026-01-19T18:00:00Z
- **Completed:** 2026-01-19T18:12:00Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- Start command with --port and --open flags, auto-builds image if missing
- Stop command with 30-second graceful shutdown timeout
- Restart command combining stop + start with single spinner for UX continuity
- CommandSpinner showing elapsed time during operations
- Idempotent behavior: start when running shows URL; stop when stopped confirms
- Pre-flight port availability check with suggestion of next available port

## Task Commits

Each task was committed atomically:

1. **Task 1: Add dependencies and create CommandSpinner** - `fa24ee4` (feat)
2. **Task 2: Implement start, stop, restart commands** - `70f300a` (feat)
3. **Task 3: Wire commands into CLI and fix API usage** - `2ca3439` (feat)

## Files Created/Modified

- `packages/cli-rust/src/output/mod.rs` - Output module exports
- `packages/cli-rust/src/output/spinner.rs` - CommandSpinner with elapsed time
- `packages/cli-rust/src/commands/mod.rs` - Commands module exports
- `packages/cli-rust/src/commands/start.rs` - Start command (235 lines)
- `packages/cli-rust/src/commands/stop.rs` - Stop command (99 lines)
- `packages/cli-rust/src/commands/restart.rs` - Restart command (116 lines)
- `packages/cli-rust/src/lib.rs` - Wired commands into CLI
- `packages/cli-rust/Cargo.toml` - Added indicatif, tokio, webbrowser, humantime, bollard
- `Cargo.toml` - Added workspace deps for webbrowser, humantime

## Decisions Made

1. **DockerClient::new() synchronous:** The core library's DockerClient::new() is synchronous and creates the client. verify_connection() is async and validates the connection. This matches bollard's design.

2. **Port availability pre-check:** Check port before attempting container creation to provide clearer error messages with suggestions for alternative ports.

3. **Quiet mode outputs URL only:** When -q flag is used, start command outputs just the URL (e.g., `http://127.0.0.1:3000`) for scripting like `open $(occ start -q)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed DockerClient API mismatch**
- **Found during:** Task 3 (wiring commands)
- **Issue:** Plan specified `DockerClient::connect()` but actual API is `DockerClient::new()` (sync) + `verify_connection()` (async)
- **Fix:** Changed to use `new()` for client creation and added `verify_connection().await` for connection validation
- **Files modified:** start.rs, stop.rs, restart.rs
- **Verification:** `cargo check` passes
- **Committed in:** `2ca3439` (Task 3 commit)

**2. [Rule 3 - Blocking] Fixed build_image signature**
- **Found during:** Task 3 (linting)
- **Issue:** Plan specified `build_image(&client, DOCKERFILE, IMAGE_TAG_DEFAULT, &progress)` but actual signature is `build_image(&client, tag: Option<&str>, progress: &mut ProgressReporter)`
- **Fix:** Changed to `build_image(&client, Some(IMAGE_TAG_DEFAULT), &mut progress)`
- **Files modified:** start.rs
- **Verification:** `cargo check` passes
- **Committed in:** `2ca3439` (Task 3 commit)

**3. [Rule 3 - Blocking] Added bollard dependency**
- **Found during:** Task 3 (linting)
- **Issue:** start.rs uses bollard types (LogOutput, LogsOptions) for showing crash logs, but bollard wasn't a direct dependency
- **Fix:** Added `bollard.workspace = true` to cli-rust Cargo.toml
- **Files modified:** packages/cli-rust/Cargo.toml
- **Verification:** `cargo check` passes
- **Committed in:** `2ca3439` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All fixes were necessary due to API signature mismatches between plan assumptions and actual core library implementation. No scope creep.

## Issues Encountered

None - plan executed smoothly after the API corrections.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Three core lifecycle commands now available via `occ start/stop/restart`
- CommandSpinner pattern established for status and logs commands
- Ready for Phase 3 Plan 2: status and logs commands
- Docker error handling patterns can be reused

---
*Phase: 03-service-lifecycle-commands*
*Completed: 2026-01-19*
