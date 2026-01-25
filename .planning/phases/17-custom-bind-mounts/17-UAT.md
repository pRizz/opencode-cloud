---
status: passed
phase: 17-custom-bind-mounts
source: 17-01-SUMMARY.md, 17-02-SUMMARY.md, 17-03-SUMMARY.md
started: 2026-01-25T20:00:00Z
completed: 2026-01-25T19:59:00Z
---

## Tests

### 1. Add Bind Mount via CLI
expected: Run `just run mount add /tmp:/mnt/tmp` — mount is added to config. Command shows confirmation message. `just run mount list` shows the mount.
result: pass

### 2. Add Read-Only Mount
expected: Run `just run mount add /tmp:/mnt/ro:ro` — read-only mount is added. `just run mount list` shows "ro" mode for this mount.
result: pass

### 3. Remove Bind Mount
expected: Run `just run mount remove /tmp` — mount is removed from config. Confirmation message shown. `just run mount list` no longer shows the mount.
result: pass

### 4. Start Container with Config Mounts
expected: After adding mount via `just run mount add`, run `just run start`. Container starts and mount is applied. `just run status` shows Mounts section with the mount marked as "(config)".
result: pass
notes: Fixed bug where macOS path translation caused config mounts to show as "(cli)" instead of "(config)". Added host_paths_match helper to status.rs.

### 5. Start with One-Time CLI Mount
expected: Run `just run start --mount /tmp:/mnt/test`. Container starts with the one-time mount. `just run status` shows the mount marked as "(cli)".
result: pass
notes: Fixed mount source detection to also check target path, distinguishing mounts with same source but different targets.

### 6. Start with --no-mounts Flag
expected: Add a mount to config, then run `just run start --no-mounts`. Container starts without config mounts applied. `just run status` shows no mounts (or only CLI mounts if --mount also used).
result: pass

### 7. Invalid Mount Path Validation
expected: Run `just run mount add /nonexistent/path:/mnt/foo`. Command fails with clear error message about path not existing.
result: pass
notes: Error message "Path not found: /nonexistent/path (No such file or directory (os error 2))" is clear.

### 8. Status Shows Active Mounts
expected: With container running and mounts applied, `just run status` displays Mounts section with host path, container path, mode (ro/rw), and source indicator (config/cli).
result: pass
notes: All mount details correctly displayed - host path (macOS-translated), container path, mode, and source indicator.

## Summary

total: 8
passed: 8
issues: 0
pending: 0
skipped: 0

## Issues Found and Fixed

1. **Mount source indicator bug (Test 4)**: Status showed config mounts as "(cli)" on macOS due to Docker path translation. Added `host_paths_match()` helper to handle `/tmp` → `/host_mnt/private/tmp` translation.

2. **Source detection false positive (Test 5)**: Mounts with same source but different targets were incorrectly marked as "(config)". Fixed by checking both source AND target paths.

## Gaps

[none]
