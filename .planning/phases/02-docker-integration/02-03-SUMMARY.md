---
phase: 02-docker-integration
plan: 03
subsystem: docker
tags: [bollard, docker, volumes, containers, lifecycle]

# Dependency graph
requires:
  - phase: 02-02
    provides: Image operations (build, pull, image_exists)
provides:
  - Named Docker volumes for persistent storage (session, projects, config)
  - Container lifecycle operations (create, start, stop, remove)
  - Volume mount configuration matching opencode paths
  - Convenience functions for full service startup/shutdown
affects: [03-cli-commands, 04-tunnel-integration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Named volumes for persistence (not bind mounts)"
    - "Localhost-only port binding for security (127.0.0.1)"
    - "Idempotent volume creation with managed-by label"

key-files:
  created:
    - packages/core/src/docker/volume.rs
    - packages/core/src/docker/container.rs
  modified:
    - packages/core/src/docker/mod.rs
    - packages/core/src/lib.rs

key-decisions:
  - "Named volumes over bind mounts for cross-platform compatibility"
  - "Port binding to 127.0.0.1 only (security: prevents external access)"
  - "Volume label 'managed-by: opencode-cloud' for identification"

patterns-established:
  - "Volume constants with corresponding mount point constants"
  - "Container creation separate from start (two-phase lifecycle)"
  - "Convenience functions wrapping multi-step operations"

# Metrics
duration: 7min
completed: 2026-01-19
---

# Phase 2 Plan 3: Volume Persistence and Container Lifecycle Summary

**Three named Docker volumes for persistence with complete container lifecycle management (create/start/stop/remove) binding to localhost:3000**

## Performance

- **Duration:** 7 min
- **Started:** 2026-01-19T17:45:00Z
- **Completed:** 2026-01-19T17:52:00Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- Volume management with three named volumes (session, projects, config)
- Container lifecycle operations with proper volume mounts
- Port binding restricted to localhost (127.0.0.1:3000) for security
- Convenience functions `setup_and_start()` and `stop_service()` for CLI use

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement volume management** - `061a037` (feat)
2. **Task 2: Implement container lifecycle operations** - `db4f356` (feat)
3. **Task 3: Wire up module exports and add convenience functions** - `6d17524` (feat)

## Files Created/Modified

- `packages/core/src/docker/volume.rs` - Volume constants and management functions (144 lines)
- `packages/core/src/docker/container.rs` - Container lifecycle operations (317 lines)
- `packages/core/src/docker/mod.rs` - Module wiring and convenience functions
- `packages/core/src/lib.rs` - Re-exports for CONTAINER_NAME and DEFAULT_PORT

## Decisions Made

1. **Named volumes over bind mounts:** Named volumes are managed by Docker and work consistently across platforms. Bind mounts would require users to specify paths and have permission issues on some systems.

2. **Localhost-only port binding (127.0.0.1):** Security measure to prevent the opencode web UI from being accessible from other machines on the network. Users who need remote access should use the tunnel feature (future phase).

3. **Managed-by label on volumes:** Added `managed-by: opencode-cloud` label to volumes for easy identification and cleanup.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed temporary value dropped while borrowed**
- **Found during:** Task 3 (verification)
- **Issue:** `format!()` result used as reference in `unwrap_or()` was dropped before use
- **Fix:** Created `let default_image = format!(...)` binding before `unwrap_or()`
- **Files modified:** packages/core/src/docker/container.rs
- **Verification:** `cargo check` passes
- **Committed in:** `6d17524` (part of Task 3 commit)

---

**Total deviations:** 1 auto-fixed (blocking)
**Impact on plan:** Minor Rust borrow-checker fix required for correctness. No scope creep.

## Issues Encountered

None - plan executed smoothly after the borrow fix.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Docker module now complete with full lifecycle management
- CLI commands can use `setup_and_start()` for `occ start` command
- CLI commands can use `stop_service()` for `occ stop` command
- Volume persistence ensures session history survives container restarts
- Ready for Phase 3 CLI command implementation

---
*Phase: 02-docker-integration*
*Completed: 2026-01-19*
