---
phase: 08-polish-and-documentation
created: 2026-01-22
status: context-gathered
---

# Phase 8 Context: Polish and Documentation

## Goal
CLI provides excellent UX with clear help and clean uninstall

## Requirements
- INST-06: User can uninstall cleanly via `opencode-cloud uninstall`
- INST-07: Clear error messages with actionable guidance
- INST-08: Help documentation available via `--help` for all commands

## User Decisions

### Uninstall Behavior
**Decision:** Service only (keep current scope), but make it clear to the user where config and other files live if they reinstall, and how to remove them manually if desired.

**Implementation:**
- Keep current uninstall behavior (service registration + optional volumes)
- After uninstall, display paths to remaining files:
  - Config: `~/.config/opencode-cloud/`
  - Data: `~/.local/share/opencode-cloud/` (if exists)
- Include instructions for manual cleanup if user wants full removal

### Help Text Depth
**Decision:** Minimal (keep current clap-derived help)

**Implementation:**
- No changes needed to help text
- Current short descriptions are sufficient
- INST-08 is already satisfied by existing `--help` output

### Error Message Format
**Decision:** Keep mixed (current state acceptable)

**Implementation:**
- No systematic changes to error handling
- INST-07 is already satisfied - config validation errors include fix commands, other errors are descriptive enough
- Focus on uninstall messaging improvements only

### Uninstall Confirmation
**Decision:** Always confirm (prompt Y/n before removing service, unless --force)

**Implementation:**
- Add confirmation prompt before any uninstall action
- `--force` skips the prompt
- Keep existing `--volumes --force` requirement for data deletion

## Current State Analysis

### Uninstall Command (packages/cli-rust/src/commands/uninstall.rs)
- 131 lines
- Removes service registration (systemd/launchd)
- `--volumes` flag removes Docker volumes (requires `--force`)
- Does NOT remove config files
- No confirmation prompt currently
- Idempotent (exits 0 if not installed)

### Help System
- Clap-derived help with `#[command(about = "...")]` attributes
- Banner displayed after main help
- All commands have basic descriptions

### Error Handling
- Uses `anyhow` throughout
- Validation errors (validation.rs) include fix commands
- Docker errors are descriptive
- Mixed consistency is acceptable per user decision

## Scope for Phase 8

### Must Do
1. Add confirmation prompt to uninstall (unless --force)
2. Display remaining file paths after uninstall completes
3. Verify all commands have `--help` (INST-08 check)

### Explicitly Out of Scope
- Adding `--config` or `--all` flags to uninstall
- Changing help text depth or adding examples
- Standardizing error message format
- Any functional changes beyond uninstall UX

## Success Criteria Mapping

| Criterion | Status | Notes |
|-----------|--------|-------|
| All commands display helpful usage via `--help` | Already done | Verify in plan |
| Error messages clear with actionable guidance | Already done | Config validation has fix commands |
| User can cleanly uninstall via `opencode-cloud uninstall` | Needs work | Add confirmation, show remaining files |
| Uninstall removes service registration, config files, optionally volumes | Mostly done | Just need to inform user about remaining files |

## Technical Notes

- Config path: `~/.config/opencode-cloud/config.json`
- Data path: `~/.local/share/opencode-cloud/` (may not exist in all installs)
- Service files: `~/.config/systemd/user/opencode-cloud.service` (Linux) or `~/Library/LaunchAgents/com.opencode.cloud.plist` (macOS)
- Docker volumes: `opencode-cloud-*` (managed by core library)
