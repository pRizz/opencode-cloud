## Phase 34: Disable Cockpit User-Facing Surface

### Goal

Remove all user-facing references to Cockpit and disable the exposed Cockpit
functionality, while preserving the underlying code so it can be re-enabled
later with minimal effort.

### Scope

- Hide Cockpit from CLI output, wizard messaging, and docs.
- Remove Cockpit endpoints/ports from deployment surfaces.
- Keep Cockpit implementation code gated or inert, not deleted.

### Non-Goals

- Removing Cockpit packages or code from the container image.
- Refactoring Cockpit internals beyond gating/visibility changes.

