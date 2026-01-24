---
phase: 15-prebuilt-image-option
plan: 02
subsystem: docker
tags: [docker, image-pull, clap, dialoguer, progress]

# Dependency graph
requires:
  - phase: 15-01
    provides: "Config schema with image_source/update_check, ImageState module"
provides:
  - "Pull-or-build choice with mutual exclusivity in start command"
  - "First-run prompt for image source preference"
  - "Pull failure fallback to build with user confirmation"
  - "Image provenance tracking via ImageState after acquisition"
affects: [15-03-wizard-integration, update-command]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Mutual exclusivity check pattern for related flags"
    - "Pull-with-fallback pattern: try pull, offer build on failure"
    - "First-run detection and preference prompt pattern"

key-files:
  created: []
  modified:
    - "packages/cli-rust/src/commands/start.rs"

key-decisions:
  - "Flag mutual exclusivity enforced at argument parsing level"
  - "Pull failures in quiet mode return error instead of prompting"
  - "First-run prompt only shown if no image exists and no flag specified"
  - "Registry extraction from full image name for provenance tracking"

patterns-established:
  - "Image flag precedence: explicit flag > config.image_source default"
  - "Version check respects both --no-update-check flag and update_check config"
  - "Container must be stopped before changing image source via flags"

# Metrics
duration: 10min
completed: 2026-01-24
---

# Phase 15 Plan 02: Pull Prebuilt Images Summary

**Pull-or-build choice with first-run prompt, mutual exclusivity, and fallback logic in start command**

## Performance

- **Duration:** 10 min
- **Started:** 2026-01-24T17:13:05Z
- **Completed:** 2026-01-24T17:22:55Z
- **Tasks:** 2
- **Files modified:** 2 (plus formatting)

## Accomplishments
- Renamed flags to --pull-sandbox-image, --cached-rebuild-sandbox-image, --full-rebuild-sandbox-image
- Implemented mutual exclusivity check for image flags
- Added first-run prompt for image source choice (pull vs build) with config persistence
- Pull failures offer fallback to build instead of hard failure
- Image provenance saved after pull or build via ImageState module

## Task Commits

Each task was committed atomically:

1. **Task 1: Rename flags and add new flags to StartArgs** - `22c0cf8` (feat - committed in prior session)
   - Note: This was already completed in a previous session but labeled incorrectly as 15-03
2. **Task 2: Implement pull-or-build logic with first-run prompt** - `ee392ab` (feat)

**Formatting:** `3cb380e` (style: rustfmt formatting fixes)

## Files Created/Modified
- `packages/cli-rust/src/commands/start.rs` - Pull-or-build logic, first-run prompt, mutual exclusivity
- `packages/cli-rust/src/commands/setup.rs` - Updated StartArgs initialization (prior session)
- `packages/cli-rust/src/wizard/mod.rs` - Type fix for total_steps (usize consistency)
- `packages/core/src/docker/state.rs` - Minor formatting

## Decisions Made

**Flag mutual exclusivity:** Only one of the three image flags (--pull-sandbox-image, --cached-rebuild-sandbox-image, --full-rebuild-sandbox-image) can be specified. This prevents confusing combinations.

**Container stop prompt:** If an image flag is used while container is running, user is prompted to stop (with --force bypass). This prevents silent failures.

**Pull failure fallback:** When pull fails (network issues, registry down), offer to build from source instead of hard failure. Improves resilience for first-time users.

**First-run prompt timing:** Only shown if no image exists AND no flag specified AND not quiet mode. This avoids interrupting automated/scripted usage.

**Registry extraction:** Extract registry name (ghcr.io, docker.io) from full image string for provenance tracking in ImageState.

**Version check behavior:** Respect both --no-update-check flag and config.update_check setting. Multiple ways to disable checks for user preference.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed type inconsistency in wizard module**
- **Found during:** Task 1 (build after flag renaming)
- **Issue:** `prompt_image_source` used `u8` for step/total while other prompt functions used `usize`, causing type mismatch
- **Fix:** Changed total_steps from `u8` to `usize` for consistency
- **Files modified:** packages/cli-rust/src/wizard/mod.rs
- **Verification:** `just build` succeeds
- **Committed in:** 3cb380e (formatting commit)

**2. [Rule 1 - Bug] Fixed clippy useless_conversion warning**
- **Found during:** Task 2 (lint check)
- **Issue:** `e.into()` conversion was unnecessary when `e` is already `anyhow::Error`
- **Fix:** Changed `return Err(e.into())` to `return Err(e)`
- **Files modified:** packages/cli-rust/src/commands/start.rs line 308
- **Verification:** `just lint` passes
- **Committed in:** ee392ab (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes necessary for compilation and lint compliance. No scope creep.

## Issues Encountered

**File modification during edit:** Encountered "file modified since read" errors during editing, likely due to rust-analyzer or rustfmt running automatically. Resolved by re-reading file before retrying edits.

**Prior session overlap:** Task 1 (flag renaming) was already completed in a prior session but committed with incorrect label (15-03 instead of 15-02). This session primarily completed Task 2.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Start command now supports pull and build paths with clear UX
- Image provenance tracking in place via ImageState
- Ready for phase 15-03 to integrate with wizard and show provenance in status/update commands
- No blockers

---
*Phase: 15-prebuilt-image-option*
*Completed: 2026-01-24*
