---
phase: 02-docker-integration
plan: 02
subsystem: docker
tags: [bollard, indicatif, progress, image-build, image-pull, tar, flate2]

# Dependency graph
requires:
  - phase: 02-01
    provides: DockerClient wrapper, DockerError types, embedded DOCKERFILE
provides:
  - Image build from embedded Dockerfile with streaming progress
  - Image pull with GHCR to Docker Hub fallback
  - Per-layer progress bars for downloads
  - image_exists check for local cache
affects: [02-03-container-lifecycle, 03-cloud-connectivity]

# Tech tracking
tech-stack:
  added: [bytes]
  patterns: [streaming-progress-bars, registry-fallback, manual-retry-loops]

key-files:
  created:
    - packages/core/src/docker/image.rs
    - packages/core/src/docker/progress.rs
  modified:
    - packages/core/src/docker/mod.rs
    - packages/core/Cargo.toml
    - Cargo.toml

key-decisions:
  - "Manual retry loop instead of tokio-retry due to async closure capture limitations"
  - "Per-layer progress bars for downloads, spinners for build steps"
  - "Exponential backoff: 1s, 2s, 4s (max 3 attempts)"

patterns-established:
  - "ProgressReporter pattern: shared progress manager with HashMap of bars/spinners"
  - "Registry fallback pattern: try GHCR first, Docker Hub as fallback"
  - "Build context pattern: create tar.gz from embedded DOCKERFILE"

# Metrics
duration: 6min
completed: 2026-01-19
---

# Phase 2 Plan 2: Image Operations Summary

**Docker image build from embedded Dockerfile and pull with GHCR/Docker Hub fallback, using indicatif progress bars for real-time layer-by-layer feedback**

## Performance

- **Duration:** 6 min
- **Started:** 2026-01-19T16:54:32Z
- **Completed:** 2026-01-19T17:00:24Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- ProgressReporter with multi-layer download tracking and build spinners
- Image build using embedded DOCKERFILE via tar.gz context
- Image pull with automatic GHCR to Docker Hub fallback
- Retry logic with exponential backoff for transient failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Create progress reporting utilities** - `a035792` (feat)
2. **Task 2: Implement image build operation** - `502a64e` (feat)
3. **Task 3: Implement image pull with registry fallback** - `e3a1ded` (feat)

## Files Created/Modified

- `packages/core/src/docker/progress.rs` - ProgressReporter with MultiProgress, spinners, and progress bars
- `packages/core/src/docker/image.rs` - build_image, pull_image, image_exists functions
- `packages/core/src/docker/mod.rs` - Module declarations and re-exports
- `packages/core/Cargo.toml` - Added bytes dependency
- `Cargo.toml` - Added bytes to workspace dependencies

## Decisions Made

- **Manual retry instead of tokio-retry:** The tokio-retry Retry::spawn closure cannot capture mutable references that outlive it. Implemented manual retry loop with same exponential backoff behavior.
- **Per-layer progress bars:** During image pull, each layer gets its own progress bar showing bytes downloaded/total. Build uses spinners since steps are indeterminate.
- **BuildInfoAux enum handling:** Bollard's aux field returns a BuildInfoAux enum with Default(ImageId) or BuildKit variants. Pattern matching extracts image ID from Default variant.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added bytes crate to dependencies**
- **Found during:** Task 2 (Image build implementation)
- **Issue:** Bollard's build_image expects Bytes type, but bytes crate wasn't in dependencies
- **Fix:** Added bytes = "1.9" to workspace and core Cargo.toml
- **Files modified:** Cargo.toml, packages/core/Cargo.toml
- **Verification:** cargo check passes
- **Committed in:** 502a64e (Task 2 commit)

**2. [Rule 1 - Bug] Fixed tokio-retry closure capture issue**
- **Found during:** Task 3 (Image pull implementation)
- **Issue:** Async closure in Retry::spawn cannot capture &mut ProgressReporter (captured variable escapes FnMut closure body)
- **Fix:** Replaced tokio-retry with manual retry loop using for loop and tokio::time::sleep
- **Files modified:** packages/core/src/docker/image.rs
- **Verification:** cargo check passes, same retry behavior
- **Committed in:** e3a1ded (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for compilation. No scope creep.

## Issues Encountered

- **Clippy needless_return:** Initial pull_image function had unnecessary return statements in match arms. Fixed by restructuring to extract error and use implicit return on final expression.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Image operations ready for container lifecycle management
- build_image, pull_image, image_exists functions exported from docker module
- Progress reporting pattern established for reuse in container operations

---
*Phase: 02-docker-integration*
*Completed: 2026-01-19*
