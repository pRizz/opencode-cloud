---
phase: 33
title: "Persist container users across updates"
status: context
updated: 2026-01-31
---

# Phase 33 Context

## Objective

Persist container users across updates/rebuilds. We chose **Option D**: store user state in a mounted directory and restore it automatically.

## Current Behavior (Problem)

- Usernames are stored in `config.users`, but passwords live only in `/etc/shadow` inside the container.
- `occ update container` recreates users (no passwords), but `occ start` after rebuild does not.
- Container filesystem is ephemeral, so user accounts disappear on recreate.

## Selected Approach (Option D)

Persist user state in a dedicated, managed volume and restore it when the container is created or started.

### Core idea

- Add a new volume (e.g., `opencode-users`) mounted at a stable path (e.g., `/var/lib/opencode-users`).
- When a user is created or updated, write a small record into that directory.
- On container (re)creation, read those records and recreate users + passwords.

### Data to store

Minimum viable record per user:

- Username
- Password hash (or full `/etc/shadow` line)
- Locked/unlocked state (if tracked)

Keep this in a root-owned path inside the container and ensure permissions are strict.

## Implementation Touchpoints

- `packages/core/src/docker/volume.rs`: add new volume constant + mount path.
- `packages/core/src/docker/container.rs`: mount the new volume when creating container.
- `packages/core/src/docker/users.rs`: add helpers to read/write persisted user records.
- `packages/cli-rust/src/commands/user/*`: ensure create/delete/lock/unlock update the persisted records.
- `packages/core/src/docker/mod.rs` (`setup_and_start`): restore users after container start.

## Constraints / Non-Goals

- Do not store plaintext passwords in host config.
- Avoid modifying external host state outside managed Docker volumes.
- Keep behavior consistent across `start`, `update`, and rebuild flows.

## Risks / Considerations

- Permissions: the persisted store must be root-owned and not world-readable.
- Compatibility: existing installs should migrate safely with no manual steps.
- Security: storing hashed passwords is still sensitive; treat like `/etc/shadow`.

## Success Criteria

- Users survive container rebuilds and updates without manual recreation.
- Passwords continue to work after update/rebuild.
- `occ start` and `occ update` show consistent user persistence behavior.
