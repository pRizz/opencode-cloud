---
status: complete
phase: 10-remote-administration-via-cockpit
source: [10-01-SUMMARY.md, 10-02-SUMMARY.md, 10-03-SUMMARY.md]
started: 2026-01-22T19:00:00Z
updated: 2026-01-23T17:35:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Container Starts with Tini (Default Mode)
expected: Run `occ start` (rebuild if needed). Container starts successfully with tini init (default). No errors. opencode web UI accessible.
result: pass

### 2. occ cockpit Shows Helpful Message
expected: Run `occ cockpit`. Since cockpit_enabled defaults to false, shows helpful message explaining Cockpit is disabled and requires Linux. Includes enable instructions.
result: pass

### 3. occ status Shows No Cockpit URL (Disabled)
expected: Run `occ status` while container is running. Since cockpit_enabled is false, Cockpit URL should NOT appear in output (or show as disabled).
result: pass

### 4. Config Options Work
expected: Run `occ config show`. Output includes cockpit_port (default 9090) and cockpit_enabled (default false). Can change with `occ config set cockpit_enabled true`.
result: pass

### 5. Config Docs Are Clear
expected: Run `occ config show` or check config file. The cockpit_enabled setting should have clear documentation that it requires Linux host and doesn't work on macOS Docker Desktop.
result: pass

### 6. Cockpit Web Console (Linux Only)
expected: [SKIP on macOS] When cockpit_enabled=true on Linux, Cockpit web UI is accessible at configured port.
result: skipped
reason: Requires Linux host - macOS Docker Desktop doesn't support systemd containers

### 7. Cockpit Terminal Access (Linux Only)
expected: [SKIP on macOS] When cockpit_enabled=true on Linux, Cockpit terminal works inside container.
result: skipped
reason: Requires Linux host - macOS Docker Desktop doesn't support systemd containers

### 8. Cockpit Services Management (Linux Only)
expected: [SKIP on macOS] When cockpit_enabled=true on Linux, can manage systemd services via Cockpit.
result: skipped
reason: Requires Linux host - macOS Docker Desktop doesn't support systemd containers

## Summary

total: 8
passed: 5
issues: 0
pending: 0
skipped: 3

## Gaps

[none yet]
