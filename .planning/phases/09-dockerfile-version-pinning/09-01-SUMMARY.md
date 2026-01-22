---
phase: 09-dockerfile-version-pinning
plan: 01
subsystem: infra
tags: [docker, version-pinning, apt, cargo, go, security]

# Dependency graph
requires:
  - phase: 02-docker-image
    provides: Base Dockerfile with tool installations
provides:
  - Version-pinned Dockerfile for reproducible builds
  - Documented version pinning policy
  - Security exception markers for critical packages
affects: [14-auto-rebuild-detection, 15-prebuilt-image-option]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - APT version wildcards (pkg=X.Y.*) for patch updates
    - Cargo exact versions (@X.Y.Z with --locked)
    - GitHub release tags (vX.Y.Z) in download URLs
    - UNPINNED comments for security-critical packages

key-files:
  created: []
  modified:
    - packages/core/src/docker/Dockerfile

key-decisions:
  - "APT wildcards allow patch updates: Use pkg=X.Y.* pattern for security patches"
  - "Security exceptions: ca-certificates, gnupg, openssh-client marked UNPINNED"
  - "Self-managing installers trusted: mise, rustup, starship, oh-my-zsh, uv, opencode"
  - "Go runtime pinned to minor: go@1.24 instead of @latest"

patterns-established:
  - "Version Pinning Policy header documents audit date and patterns"
  - "Date headers on package groups (2026-01-22) for audit tracking"
  - "Inline comments with version and date for GitHub/cargo/go tools"

# Metrics
duration: 5min
completed: 2026-01-22
---

# Phase 9 Plan 1: Dockerfile Version Pinning Summary

**Pinned all Dockerfile tools to explicit versions with APT wildcards, cargo @version syntax, and GitHub release tags for reproducible builds**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-22T16:23:41Z
- **Completed:** 2026-01-22T16:28:38Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- All APT packages use version wildcards (e.g., `git=1:2.43.*`) allowing security patches
- Security-critical packages marked with `# UNPINNED:` comments (ca-certificates, gnupg, openssh-client)
- All cargo installs pinned: ripgrep@15.1.0, eza@0.23.4, cargo-nextest@0.9.123, cargo-audit@0.22.0, cargo-deny@0.19.0
- All Go tools pinned: lazygit@v0.58.1, shfmt@v3.12.0, grpcurl@v1.9.3
- GitHub releases pinned: fzf v0.67.0, yq v4.50.1, act v0.2.84
- Added Version Pinning Policy documentation header

## Task Commits

Each task was committed atomically:

1. **Task 1: Pin APT packages with version wildcards** - `3e37009` (feat)
2. **Task 2: Pin GitHub tools and cargo/go installs with version tags** - `d69f23e` (feat)
3. **Task 3: Add inline documentation and verify build** - Verification only (no changes)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified

- `packages/core/src/docker/Dockerfile` - Version-pinned with inline documentation

## Decisions Made

1. **APT wildcard pattern**: Use `pkg=X.Y.*` for major.minor wildcards allowing patch updates
2. **Security exceptions**: Mark ca-certificates, gnupg, openssh-client as UNPINNED for auto-updates
3. **Self-managing installers**: Trust mise, rustup, starship, oh-my-zsh, uv, opencode to handle their own versions
4. **Go runtime pinning**: Pin to go@1.24 (minor version) instead of @latest
5. **pnpm pinning**: Pin to pnpm@10.28.1 instead of @latest

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Dockerfile now produces reproducible builds
- Ready for Phase 10 (Remote Administration via Cockpit)
- Version audit date documented (2026-01-22) for future update checks

---
*Phase: 09-dockerfile-version-pinning*
*Completed: 2026-01-22*
