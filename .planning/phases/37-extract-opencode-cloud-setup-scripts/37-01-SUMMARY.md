---
phase: 37-extract-opencode-cloud-setup-scripts
plan: 01
subsystem: infra
tags: [aws, cloudformation, cloud-init, bash, provisioning]

# Dependency graph
requires:
  - phase: 33
    provides: baseline opencode-cloud provisioning flow and AWS templates
provides:
  - shared provisioning scripts under scripts/provisioning
  - AWS templates bootstrapped to fetch shared scripts
affects: [cloud provisioning, phase-20, non-aws providers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Bootstrap fetches provisioning scripts from a pinned Git ref with checksum verification"
    - "Shared core setup script with provider-specific wrappers"

key-files:
  created:
    - scripts/provisioning/opencode-cloud-setup.sh
    - scripts/provisioning/opencode-cloud-setup-cloudformation.sh
    - scripts/provisioning/opencode-cloud-setup-cloud-init.sh
  modified:
    - infra/aws/cloudformation/opencode-cloud-quick.yaml
    - infra/aws/cloud-init/opencode-cloud-quick.yaml

key-decisions:
  - "Split provisioning into shared core + cloudformation/cloud-init wrappers to isolate AWS-only behavior"
  - "Pin bootstrap downloads to a specific Git commit and verify sha256 checksums"

patterns-established:
  - "Provisioning scripts live in repo and are fetched by cloud templates"

# Metrics
duration: 9 min
completed: 2026-02-02
---

# Phase 37 Plan 01 Summary

**Provisioning extracted into shared scripts with AWS templates bootstrapping from a pinned repo ref and checksum verification.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-02-02T14:00:06Z
- **Completed:** 2026-02-02T14:09:39Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments
- Documented and codified the shared/core vs AWS-specific provisioning split
- Implemented shared setup script plus cloudformation/cloud-init wrappers
- Rewired AWS templates to fetch pinned scripts instead of embedding long bodies

## Task Commits

Each task was committed atomically:

1. **Task 1: Audit current setup scripts and decide split** - `324d11e` (docs)
2. **Task 2: Create shared provisioning scripts** - `3f95095` (feat)
3. **Task 3: Update AWS templates to use repo scripts** - `9e87d7c` (feat)

**Plan metadata:** _pending_

## Files Created/Modified
- `scripts/provisioning/opencode-cloud-setup.sh` - shared core provisioning logic and layout docs
- `scripts/provisioning/opencode-cloud-setup-cloudformation.sh` - CloudFormation wrapper with signal + secrets handling
- `scripts/provisioning/opencode-cloud-setup-cloud-init.sh` - cloud-init wrapper with status/motd output
- `infra/aws/cloudformation/opencode-cloud-quick.yaml` - bootstrap fetch for shared scripts
- `infra/aws/cloud-init/opencode-cloud-quick.yaml` - bootstrap fetch for shared scripts

## Decisions Made
- Split provisioning into shared core plus cloudformation/cloud-init wrappers to isolate AWS-only behaviors while keeping shared logic reusable.
- Pin bootstrap downloads to a specific Git commit and verify sha256 checksums to avoid inline scripts and preserve integrity.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Shared provisioning flow ready for reuse outside AWS
- AWS templates now fetch from repo scripts without inline duplication

---
*Phase: 37-extract-opencode-cloud-setup-scripts*
*Completed: 2026-02-02*
