---
phase: 35-can-we-add-a-new-update-subcommand-to-update-the-opencode-cloud-binary-itself-we-might-need-to-be-careful-with-respect-to-how-the-binary-was-installed-with-which-package-manager-and-whether-it-is-the-rust-or-npm-binary
verified: 2026-01-31T23:45:39Z
status: passed
score: 3/3 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 2/3
  gaps_closed:
    - "Users get a clear error when install method cannot be detected"
  gaps_remaining: []
  regressions: []
---

# Phase 35: Can we add a new update subcommand to update the opencode-cloud binary itself? We might need to be careful with respect to how the binary was installed with which package manager and whether it is the rust or npm binary Verification Report

**Phase Goal:** Add a new update subcommand that updates the opencode-cloud binary itself, choosing the correct installation method (cargo or npm) when possible and restarting the service afterward. Provide a clear error/help message when the install method cannot be detected.
**Verified:** 2026-01-31T23:45:39Z
**Status:** passed
**Re-verification:** Yes — after gap closure

## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | Update CLI can re-install opencode-cloud when the install method is known | ✓ VERIFIED | `cmd_update_cli` maps detected install method to `cargo install opencode-cloud` or `npm install -g opencode-cloud`. |
| 2 | Users get a clear error when install method cannot be detected | ✓ VERIFIED | `cmd_update_cli` prints guidance with cargo/npm commands and a note for other package managers when detection fails. |
| 3 | Service restarts after a successful opencode-cloud update | ✓ VERIFIED | `cmd_update_cli` calls `cmd_restart` after update success. |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `packages/cli-rust/src/commands/update.rs` | Update subcommand flow for opencode-cloud binary | ✓ VERIFIED | Exists, substantive implementation, wired via `UpdateCommand::Cli`. |
| `README.md` | User-facing instructions for updating the CLI | ✓ VERIFIED | Usage includes `occ update cli`. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `cmd_update_cli` | `InstallMethod::run_update` | `detect_install_method()` | ✓ WIRED | Detects method and runs cargo/npm update command. |
| `cmd_update_cli` | `cmd_restart` | restart call | ✓ WIRED | Service restart invoked after update success. |
| `cmd_update_cli` | error guidance | `detect_install_method()` fallback | ✓ WIRED | Guidance includes cargo/npm commands and alternate-package-manager note. |

### Requirements Coverage

| Requirement | Status | Blocking Issue |
| ----------- | ------ | -------------- |
| No Phase 35 requirements found in `REQUIREMENTS.md` | N/A | N/A |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| None | - | - | - | - |

### Human Verification Required

None.

### Gaps Summary

No gaps found. All must-haves are verified.

---

_Verified: 2026-01-31T23:45:39Z_
_Verifier: Claude (gsd-verifier)_
