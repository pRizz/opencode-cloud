---
status: testing
phase: 11-remote-host-management
source: [11-01-SUMMARY.md, 11-02-SUMMARY.md, 11-03-SUMMARY.md]
started: 2026-01-23T19:00:00Z
updated: 2026-01-23T19:00:00Z
---

## Current Test

number: 1
name: Host Add with --no-verify
expected: |
  Run: `just run host add test-host example.com --no-verify`
  Expected: Shows "Added: Host 'test-host' added (example.com)." with a note about unverified connection.
awaiting: user response

## Tests

### 1. Host Add with --no-verify
expected: Run `just run host add test-host example.com --no-verify`. Shows "Added: Host 'test-host' added (example.com)." with a note about unverified connection.
result: [pending]

### 2. Host List Display
expected: Run `just run host list`. Displays a table with columns: Name, Hostname, User, Port, Groups, Default. Shows test-host entry.
result: [pending]

### 3. Host Show Details
expected: Run `just run host show test-host`. Displays host configuration (hostname, user, port). Run `just run host show test-host --json` outputs JSON format.
result: [pending]

### 4. Host Edit Partial Update
expected: Run `just run host edit test-host --port 2222`. Shows "Updated: Host 'test-host' updated." Then `just run host show test-host` confirms port is 2222.
result: [pending]

### 5. Host Default Management
expected: Run `just run host default test-host` sets the default. Run `just run host default` (no args) shows "Default host: test-host". Run `just run host default local` clears it.
result: [pending]

### 6. Host Remove with Confirmation
expected: Run `just run host remove test-host`. Prompts "Remove host 'test-host'?". Answer 'n' cancels. Run `just run host remove test-host --force` removes without prompting.
result: [pending]

### 7. Global --host Flag Visible
expected: Run `just run -- --help`. Shows `--host <HOST>` as a global option. Run `just run start --help` also shows `--host` option.
result: [pending]

### 8. Host Names-Only for Scripting
expected: Run `just run host list --names-only`. Outputs only host names (one per line, no table) for shell scripting.
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
skipped: 0

## Gaps

[none yet]
