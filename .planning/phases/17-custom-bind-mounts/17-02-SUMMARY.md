---
phase: 17-custom-bind-mounts
plan: 02
subsystem: cli
tags: [clap, comfy-table, bind-mounts, cli-subcommands]

# Dependency graph
requires:
  - phase: 17-01
    provides: ParsedMount struct, validate_mount_path, check_container_path_warning
provides:
  - occ mount add command for adding bind mounts to config
  - occ mount remove command for removing bind mounts from config
  - occ mount list command for displaying configured mounts
  - --mount flag on occ start for one-time mounts
  - --no-mounts flag on occ start to skip config mounts
affects: [17-03, container-integration]

# Tech tracking
tech-stack:
  added: []
  patterns: [clap-subcommand-group, config-save-workflow]

key-files:
  created:
    - packages/cli-rust/src/commands/mount/mod.rs
    - packages/cli-rust/src/commands/mount/add.rs
    - packages/cli-rust/src/commands/mount/remove.rs
    - packages/cli-rust/src/commands/mount/list.rs
  modified:
    - packages/cli-rust/src/commands/mod.rs
    - packages/cli-rust/src/lib.rs
    - packages/cli-rust/src/commands/start.rs
    - packages/cli-rust/src/commands/setup.rs

key-decisions:
  - "Mount group follows user/ pattern: Subcommand group with MountArgs, MountCommands enum"
  - "Duplicate detection by host path: Adding same host path twice is idempotent, not an error"
  - "Restart note in output: CLI reminds users to restart container for changes to take effect"

patterns-established:
  - "Subcommand group pattern: mod.rs with Args/Commands enum, subcommand files export FooArgs and cmd_foo"
  - "Config persistence workflow: load_config -> modify -> save_config with validation"

# Metrics
duration: 3min
completed: 2026-01-25
---

# Phase 17 Plan 02: Mount CLI Subcommand Summary

**CLI mount subcommand group (add/remove/list) and start command --mount/--no-mounts flags**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-25T18:15:11Z
- **Completed:** 2026-01-25T18:18:19Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Created `occ mount` subcommand group following the user/ pattern
- `occ mount add /host:/container[:ro]` adds mount to config with validation
- `occ mount remove /host/path` removes mount by host path from config
- `occ mount list` displays mounts in table format with --names-only scripting mode
- `occ start --mount /a:/b` flag for one-time mounts (parsed, integration in plan 03)
- `occ start --no-mounts` flag to skip config mounts (parsed, integration in plan 03)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create mount subcommand group structure** - `976f6eb` (feat)
2. **Task 2: Register mount subcommand and add start flags** - `b27d3ce` (feat)

## Files Created/Modified
- `packages/cli-rust/src/commands/mount/mod.rs` - MountArgs, MountCommands enum, cmd_mount handler
- `packages/cli-rust/src/commands/mount/add.rs` - cmd_mount_add with parse, validate, save_config
- `packages/cli-rust/src/commands/mount/remove.rs` - cmd_mount_remove with load, filter, save
- `packages/cli-rust/src/commands/mount/list.rs` - cmd_mount_list with table output using comfy_table
- `packages/cli-rust/src/commands/mod.rs` - Added mod mount and pub use mount exports
- `packages/cli-rust/src/lib.rs` - Added Mount variant to Commands enum and match arm
- `packages/cli-rust/src/commands/start.rs` - Added --mount and --no-mounts flags to StartArgs
- `packages/cli-rust/src/commands/setup.rs` - Updated StartArgs initialization with new fields

## Decisions Made
- Mount subcommand group follows user/ pattern for consistency
- Duplicate host path detection is idempotent (shows message but no error)
- CLI reminds users to restart container for changes to take effect

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated setup.rs StartArgs initialization**
- **Found during:** Task 2 (register mount subcommand)
- **Issue:** setup.rs creates StartArgs without new mounts/no_mounts fields
- **Fix:** Added mounts: Vec::new() and no_mounts: false to StartArgs initialization
- **Files modified:** packages/cli-rust/src/commands/setup.rs
- **Verification:** cargo build passes
- **Committed in:** b27d3ce (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Auto-fix necessary for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Mount CLI commands working with config persistence
- Ready for plan 03 to integrate mounts into container creation
- --mount and --no-mounts flags parsed and available in StartArgs

---
*Phase: 17-custom-bind-mounts*
*Completed: 2026-01-25*
