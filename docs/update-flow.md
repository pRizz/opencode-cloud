---
title: Webapp Update Flow (Host Command File)
---

# Webapp Update Flow (Host Command File)

This document explains how a webapp running inside the opencode container can trigger a host-side
`occ update opencode` via a bind-mounted command file. It is written for implementors of the
opencode webapp and describes the expected file paths, JSON payloads, error cases, and recommended
write behavior.

## Overview

The host runs `occ start --no-daemon` as a foreground service (via `occ install`). When running,
it polls a bind-mounted command file. The webapp writes a JSON command to that file, and the host
executes `occ update opencode`. A result JSON is written back to a sibling file.

Key points:
- No network port is required.
- Access control is filesystem permissions on the bind mount.
- The listener runs only when `occ` is running in foreground (service mode).

## Service Requirement

The listener is started when `occ start --no-daemon` runs. This happens automatically when the
service is installed:

```
occ install
```

If `occ` is running in the background or not running as a service, the command file will not be
processed. Ensure the service is installed and running.

## Paths and Bind Mounts

The command file lives under the state bind mount. With default mounts, the paths are:

- Host: `~/.local/state/opencode/opencode-cloud/commands/update-command.json`
- Container: `/home/opencoder/.local/state/opencode/opencode-cloud/commands/update-command.json`
- Result file (host): `~/.local/state/opencode/opencode-cloud/commands/update-command.result.json`
- Result file (container): `/home/opencoder/.local/state/opencode/opencode-cloud/commands/update-command.result.json`

If you customize mounts, the container path is still under:

```
/home/opencoder/.local/state/opencode/opencode-cloud/commands/
```

## Command File Contract

### Request JSON (webapp writes)

```json
{
  "command": "update_opencode",
  "request_id": "optional-id",
  "branch": "dev",
  "commit": "optional-sha"
}
```

Fields:
- `command` (required): must be `"update_opencode"`.
- `request_id` (optional): any string to correlate request and result.
- `branch` (optional): update to a branch (e.g., `"dev"`).
- `commit` (optional): update to a commit SHA.

Rules:
- Specify either `branch` or `commit`, not both.
- If both are omitted, the host uses the default behavior (currently `dev`).

### Result JSON (host writes)

```json
{
  "status": "success",
  "request_id": "optional-id",
  "message": "Update completed",
  "started_at": "2026-02-01T22:48:00Z",
  "finished_at": "2026-02-01T22:48:12Z"
}
```

Fields:
- `status`: `"success"` or `"error"`.
- `request_id`: echoed from the request when provided.
- `message`: human-readable status or error message.
- `started_at` / `finished_at`: ISO-8601 timestamps.

## Recommended Write Behavior (Webapp)

To avoid partial reads:

1. Write to a temporary file in the same directory.
2. Atomically rename to `update-command.json`.

Example pseudo-steps:

```
write /commands/update-command.json.tmp
rename to /commands/update-command.json
```

Then poll the result file until it exists and matches the `request_id`.

## Error Cases and Scenarios

### Rate Limit (Burst Protection)
The listener processes at most one command every 5 seconds. If multiple commands are written
back-to-back, the first one is handled and the next one waits until the interval elapses.

### Command File Is Missing
If the command file does not exist, the host does nothing. This is normal.

### Invalid JSON
If the JSON is invalid and the file is older than a short grace window, the host will:
- record an error result in the result file
- delete the command file

If invalid JSON is detected immediately after write, the host will retry once the file
stabilizes (to avoid race conditions).

### Unsupported Command
If `command` is not `"update_opencode"`, the host writes an error result and removes the file.

### Both `branch` and `commit` Set
This is rejected with an error result.

### Missing Bind Mount or Read-Only Mount
If the state mount is missing or read-only, the listener disables itself. The webapp will never
see a response. Ensure the default mounts are enabled and writable.

### Host Not Running in Service Mode
If the host service is not running (or running without `--no-daemon`), the listener is not active.
The command file will not be processed.

### Container Not Running
If the container is stopped, the listener exits. The command file will not be processed until the
service is restarted.

### Update Already Up To Date
If opencode is already at the requested commit, the update command returns success and writes a
result indicating no change.

### Update Failure
Any update failure writes a `"status": "error"` result with the failure message. The webapp should
surface this to the user.

## UI/UX Suggestions

Recommended states for the webapp button:
- Idle: show "Update opencode".
- Pending: disable button, show "Updating..." and poll for result.
- Success: show "Updated" with timestamp.
- Error: show error message and allow retry.

## Alternatives (Context)

These are not implemented but are viable options for future work:

1. **Host HTTP API endpoint**
   - A local `POST /api/v1/update` endpoint with JSON payload.
   - Easier streaming updates, but requires network exposure and auth.

2. **Unix socket + proxy**
   - HTTP over a unix socket with a local reverse proxy in the container.
   - Strong local permissions but more moving parts.

3. **Docker socket from container**
   - Webapp triggers a helper that calls Docker directly.
   - Not recommended due to high privileges in the container.

## Runtime Parity Refactor Checklist

This project now has a shared runtime command core in
`packages/cli-rust/src/commands/runtime_shared/` to prevent host/container drift.

- [x] Move status health mapping + probes into shared runtime core.
- [x] Route host/container status implementations through shared status model.
- [x] Reuse shared broker readiness semantics in startup readiness checks.
- [ ] Migrate logs command builder/normalization into shared runtime core.
- [ ] Migrate user command domain flow into shared runtime core.
- [ ] Migrate `update opencode` shared checks/restart semantics into shared runtime core.
