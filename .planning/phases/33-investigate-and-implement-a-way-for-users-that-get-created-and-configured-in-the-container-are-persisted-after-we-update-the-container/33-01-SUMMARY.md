---
phase: 33-investigate-and-implement-a-way-for-users-that-get-created-and-configured-in-the-container-are-persisted-after-we-update-the-container
plan: 01
subsystem: auth
tags: [docker, volume, pam, users]

# Dependency graph
requires:
  - phase: 06-security-and-authentication
    provides: container user management and PAM authentication flows
  - phase: 07-update-and-maintenance
    provides: container update and rebuild workflows
provides:
  - managed user persistence volume with restore logic for container accounts
affects: [start, update, user-management]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Persist container user state in a managed Docker volume"]

key-files:
  created: []
  modified:
    - packages/core/src/docker/users.rs
    - packages/core/src/docker/volume.rs
    - packages/core/src/docker/container.rs
    - packages/core/src/docker/mod.rs
    - packages/cli-rust/src/commands/user/add.rs
    - packages/cli-rust/src/commands/user/remove.rs
    - packages/cli-rust/src/commands/user/passwd.rs
    - packages/cli-rust/src/commands/user/enable.rs
    - packages/cli-rust/src/commands/update.rs
    - README.md

key-decisions:
  - "Persist users as per-user JSON records with shadow hashes in a managed volume"

patterns-established:
  - "User mutations update a persistence record to survive rebuilds"

# Metrics
duration: 20 min
completed: 2026-02-01
---

# Phase 33 Plan 01 Summary

**Persisted container users and passwords across rebuilds using a dedicated volume and automatic restore flow.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-02-01T01:23:51Z
- **Completed:** 2026-02-01T01:43:51Z
- **Tasks:** 4
- **Files modified:** 10

## Accomplishments
- Added a managed user persistence volume and container mount for stored credentials.
- Implemented JSON-backed user record persistence and restore logic tied to container start.
- Wired user mutation commands to update persisted records and documented behavior.

## Task Commits

Each task was committed atomically:

1. **Task 1: Define persistence store and format** - `feat(33-01): persist user records` (`1589479`)
2. **Task 2: Add volume and mount for user store** - `feat(33-01): add user persistence volume` (`269d8a2`)
3. **Task 3: Persist and restore users in CLI flows** - `feat(33-01): persist user mutations` (`18ee551`)
4. **Task 4: Documentation and warnings** - `docs(33-01): document user persistence` (`0899646`)

**Plan metadata:** Pending

## Files Created/Modified
- `packages/core/src/docker/users.rs` - Persist/restore user records with shadow hashes
- `packages/core/src/docker/volume.rs` - Add managed user volume and mount constant
- `packages/core/src/docker/container.rs` - Mount user persistence volume on container creation
- `packages/core/src/docker/mod.rs` - Restore persisted users during setup/start
- `packages/cli-rust/src/commands/user/add.rs` - Persist new user records on creation
- `packages/cli-rust/src/commands/user/remove.rs` - Remove persisted records on deletion
- `packages/cli-rust/src/commands/user/passwd.rs` - Persist password updates
- `packages/cli-rust/src/commands/user/enable.rs` - Persist enable/disable state changes
- `packages/cli-rust/src/commands/update.rs` - Align update messaging with password persistence
- `README.md` - Document user persistence volume behavior

## Decisions Made
- Persist user accounts in `/var/lib/opencode-users` with per-user JSON records storing shadow hashes and lock status to avoid plaintext credentials.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Seed persistence store from existing users when empty**
- **Found during:** Task 3 (Persist and restore users in CLI flows)
- **Issue:** Existing installs would have no persisted records, so users could still be lost on first update.
- **Fix:** When no records exist, seed the store from current container users.
- **Files modified:** `packages/core/src/docker/users.rs`
- **Verification:** Store is populated on first start, enabling later restores.
- **Committed in:** `feat(33-01): persist user records` (`1589479`)

**2. [Rule 1 - Auto-fix Bug] Update messaging still warned about password loss**
- **Found during:** Task 3 (Persist and restore users in CLI flows)
- **Issue:** `occ update container` continued to warn about password resets even though passwords now persist.
- **Fix:** Removed outdated warnings and updated step counts.
- **Files modified:** `packages/cli-rust/src/commands/update.rs`
- **Committed in:** `fix(33-01): align update messaging with persistence` (`a76f6ea`)

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** Required for safe migration of existing installs; no scope creep.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- User persistence implemented with managed volume and restore flow.
- Ready to proceed to the next phase or perform manual verification.

---
*Phase: 33-investigate-and-implement-a-way-for-users-that-get-created-and-configured-in-the-container-are-persisted-after-we-update-the-container*
*Completed: 2026-02-01*
