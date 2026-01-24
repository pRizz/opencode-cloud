---
phase: 15-prebuilt-image-option
plan: 01
subsystem: config
tags: [config-schema, image-state, prebuilt-images, provenance-tracking]

# Dependency graph
requires:
  - phase: 14-versioning-and-release-automation
    provides: Version detection and image labeling infrastructure
provides:
  - Config fields for image source preference (prebuilt vs build)
  - Config field for update check frequency (always, once, never)
  - Image state module for tracking image provenance
  - Backward-compatible config migration for existing users
affects: [15-02, 15-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config migration with serde defaults for backward compatibility"
    - "Image state tracking in JSON file at data directory"

key-files:
  created:
    - packages/core/src/docker/state.rs
  modified:
    - packages/core/src/config/schema.rs
    - packages/core/src/docker/mod.rs

key-decisions:
  - "Image source defaults to 'prebuilt' for new users"
  - "Update check defaults to 'always' for automatic security updates"
  - "Image state stored in data directory (not config) as operational state"
  - "State file uses ISO8601 timestamps via chrono::Utc"

patterns-established:
  - "Image provenance tracking: version, source, registry, acquisition timestamp"
  - "State management separate from config (ephemeral vs persistent)"

# Metrics
duration: 3min
completed: 2026-01-24
---

# Phase 15 Plan 01: Config Schema & State Tracking Summary

**Config accepts image_source/update_check fields with defaults; image state module tracks provenance in ~/.local/share/opencode-cloud/image-state.json**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-24T23:12:23Z
- **Completed:** 2026-01-24T23:15:28Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added image_source config field (default: "prebuilt")
- Added update_check config field (default: "always")
- Created ImageState module with save/load/clear functions
- Full backward compatibility for existing configs

## Task Commits

Each task was committed atomically:

1. **Task 1: Add image_source and update_check to config schema** - `d239cdd` (feat)
2. **Task 2: Create image state module for provenance tracking** - `9279d3b` (feat)

## Files Created/Modified
- `packages/core/src/config/schema.rs` - Added image_source and update_check fields with serde defaults
- `packages/core/src/docker/state.rs` - New module for image provenance tracking
- `packages/core/src/docker/mod.rs` - Export state module public API

## Decisions Made

**Image source default: "prebuilt"**
- Rationale: Most users want fast setup; pull from GHCR is faster than local build
- Can be changed to "build" for users wanting reproducible builds

**Update check default: "always"**
- Rationale: Security patches should be discovered automatically
- Users can opt-out with "once" (once per version) or "never"

**State file location: data directory**
- Rationale: Image state is operational/ephemeral (which image is current)
- Config directory reserved for user preferences
- Uses get_data_dir() → ~/.local/share/opencode-cloud/image-state.json

**ISO8601 timestamps via chrono::Utc**
- Rationale: Standardized format, timezone-aware, sortable
- Uses to_rfc3339() for acquisition timestamp

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Phase 15-02 (Pull Prebuilt Images):
- Config schema supports image_source preference
- State module can track where images came from
- Backward compatibility ensures existing users unaffected

---
*Phase: 15-prebuilt-image-option*
*Completed: 2026-01-24*
