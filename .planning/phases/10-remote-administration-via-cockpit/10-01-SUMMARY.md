---
phase: 10-remote-administration-via-cockpit
plan: 01
subsystem: infra
tags: [systemd, cockpit, docker, init, container]

# Dependency graph
requires:
  - phase: 09-dockerfile-version-pinning
    provides: Pinned package versions for reproducible builds
provides:
  - systemd as PID 1 for container init
  - Cockpit web console installed and enabled
  - opencode.service systemd unit
  - Cockpit configuration with HTTP and proxy headers
affects: [10-02, 10-03, container runtime, future container changes]

# Tech tracking
tech-stack:
  added: [systemd, dbus, cockpit-ws, cockpit-system, cockpit-bridge]
  patterns: [systemd socket activation, systemd service units, cgroup volumes]

key-files:
  modified:
    - packages/core/src/docker/Dockerfile

key-decisions:
  - "systemd as PID 1: Required for Cockpit socket activation"
  - "Minimal Cockpit packages: cockpit-ws, cockpit-system, cockpit-bridge"
  - "AllowUnencrypted=true: TLS terminated externally like opencode"
  - "Keep tini in image: Backward compatibility, though systemd now default"
  - "STOPSIGNAL SIGRTMIN+3: Proper systemd shutdown signal"

patterns-established:
  - "systemd service for opencode: /etc/systemd/system/opencode.service"
  - "Cockpit socket activation: systemctl enable cockpit.socket"
  - "VOLUME for systemd: /sys/fs/cgroup, /run, /tmp"
  - "Masked services: dev-hugepages.mount, sys-fs-fuse-connections.mount, etc."

# Metrics
duration: 4min
completed: 2026-01-22
---

# Phase 10 Plan 01: Dockerfile Cockpit Integration Summary

**systemd as container init with Cockpit web console and opencode.service for integrated remote administration**

## Performance

- **Duration:** 4 min
- **Started:** 2026-01-22T18:35:11Z
- **Completed:** 2026-01-22T18:38:52Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Switched container init from tini to systemd for Cockpit support
- Installed Cockpit web console (cockpit-ws, cockpit-system, cockpit-bridge)
- Created opencode.service systemd unit with proper PATH and restart config
- Configured Cockpit for HTTP with proxy header support
- Exposed port 9090 for Cockpit alongside port 3000 for opencode

## Task Commits

Each task was committed atomically:

1. **Task 1: Add systemd and dbus packages** - `eefe2eb` (feat)
2. **Task 2: Add Cockpit packages from standard Ubuntu repos** - `cdd0cab` (feat)
3. **Task 3: Create opencode systemd service and switch to systemd init** - `53d20b6` (feat)

## Files Created/Modified

- `packages/core/src/docker/Dockerfile` - Added systemd packages, Cockpit installation, opencode.service unit, and switched to systemd init

## Decisions Made

1. **Standard Ubuntu repos for Cockpit:** Ubuntu noble has cockpit 316 in main repos which provides all needed functionality. Avoided backports for reliability.

2. **Minimal Cockpit package set:** Installed only cockpit-ws, cockpit-system, cockpit-bridge. This provides system overview, terminal access, service management, and logs viewer (~50MB additional).

3. **AllowUnencrypted=true:** Cockpit configured for HTTP since TLS is terminated externally (same pattern as opencode).

4. **Proxy header support:** Added ProtocolHeader and ForwardedForHeader in cockpit.conf for reverse proxy scenarios.

5. **Keep tini in image:** Retained tini/dumb-init for backward compatibility, though systemd is now the default init.

6. **SIGRTMIN+3 for STOPSIGNAL:** Proper systemd shutdown signal for clean container termination.

7. **Increased HEALTHCHECK start-period:** Changed from 5s to 30s because systemd takes longer to initialize than tini.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed as specified.

## User Setup Required

None - no external service configuration required. Cockpit is installed and enabled automatically in the container.

## Next Phase Readiness

- Dockerfile now has systemd as PID 1 with Cockpit and opencode.service enabled
- Ready for 10-02 (Container Runtime Configuration) to add systemd container flags
- Ready for 10-03 (CLI Integration) to add cockpit_port config and occ cockpit command
- Container will need to be rebuilt for changes to take effect

---
*Phase: 10-remote-administration-via-cockpit*
*Completed: 2026-01-22*
