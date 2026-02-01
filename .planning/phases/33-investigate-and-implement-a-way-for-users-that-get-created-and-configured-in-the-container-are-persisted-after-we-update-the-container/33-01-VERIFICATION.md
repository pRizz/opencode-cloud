---
phase: 33-investigate-and-implement-a-way-for-users-that-get-created-and-configured-in-the-container-are-persisted-after-we-update-the-container
verified: 2026-02-01T01:50:31Z
status: human_needed
score: 3/3 must-haves verified
human_verification:
  - test: "Create a container user, run `occ update container`, then attempt login"
    expected: "User still exists and can authenticate with the same password"
    why_human: "Requires actual container update/rebuild and auth check"
  - test: "Run `occ start` after removing container, then verify user list"
    expected: "Previously created users are restored automatically"
    why_human: "Depends on Docker lifecycle behavior not exercised in static checks"
  - test: "Disable then re-enable a user, update container, then login"
    expected: "Lock state and password behavior persist across update"
    why_human: "Lock state restoration and login flow are runtime behaviors"
---

# Phase 33: Investigate and implement a way for users that get created and configured in the container are persisted after we update the container Verification Report

**Phase Goal:** Persist container users and passwords across rebuilds and updates  
**Verified:** 2026-02-01T01:50:31Z  
**Status:** human_needed  
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | User accounts survive container rebuilds and updates | ✓ VERIFIED | Managed volume `opencode-users` mounted at `/var/lib/opencode-users`, restore invoked during `setup_and_start` |
| 2 | Passwords continue working after rebuild/update | ✓ VERIFIED | Shadow hash persisted and restored via `usermod -p` in `restore_persisted_users` |
| 3 | `occ start` and `occ update` behave consistently for users | ✓ VERIFIED | Both flows call `setup_and_start`, which restores persisted users after start |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `packages/core/src/docker/volume.rs` | New managed volume for user persistence | ✓ VERIFIED | Defines `VOLUME_USERS` and `MOUNT_USERS` and includes in `VOLUME_NAMES` |
| `packages/core/src/docker/users.rs` | Persist/restore user records | ✓ VERIFIED | Implements JSON-backed persistence with shadow hashes and restore flow |
| `packages/core/src/docker/mod.rs` | Restore users on setup/start | ✓ VERIFIED | `setup_and_start` calls `restore_persisted_users` after start |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `create_container` | User persistence volume | Docker volume mount | ✓ WIRED | `MOUNT_USERS` → `VOLUME_USERS` |
| `setup_and_start` | `restore_persisted_users` | Post-start restore call | ✓ WIRED | Restore invoked after container start |
| `occ user add/passwd/enable/disable/remove` | Persisted store | `persist_user` / `remove_persisted_user` | ✓ WIRED | User mutations update store |
| `occ update container` | Restore users | `setup_and_start` | ✓ WIRED | Update recreates container then restores |

### Requirements Coverage

| Requirement | Status | Blocking Issue |
| ----------- | ------ | -------------- |
| No phase-mapped requirements found in `REQUIREMENTS.md` | N/A | N/A |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| None | - | - | - | - |

### Human Verification Required

### 1. Update preserves login

**Test:** Create a user, run `occ update container`, then log in via web UI or Cockpit  
**Expected:** User still exists and password works  
**Why human:** Requires live container update and auth verification

### 2. Recreate container preserves users

**Test:** Stop and remove the container, run `occ start`, then list users  
**Expected:** Users are restored from persistence volume  
**Why human:** Docker lifecycle behavior requires runtime validation

### 3. Lock state persists across update

**Test:** Disable a user, update container, then attempt login; re-enable and retry  
**Expected:** Disabled users stay locked until re-enabled; password still works  
**Why human:** Lock state and auth are runtime behaviors

### Gaps Summary

All structural checks for persistence and restore wiring are present. Manual verification is still needed to confirm the behavior during real container rebuild/update flows.

---

_Verified: 2026-02-01T01:50:31Z_  
_Verifier: Claude (gsd-verifier)_
