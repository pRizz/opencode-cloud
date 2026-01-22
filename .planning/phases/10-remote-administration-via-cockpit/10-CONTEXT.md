# Phase 10: Remote Administration via Cockpit - Context

**Gathered:** 2026-01-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Integrate Cockpit into the Docker container to provide a web-based admin interface for managing the containerized environment. Users can manage services, access terminal, and monitor system status via Cockpit, complementing the CLI for those who prefer GUI access.

</domain>

<decisions>
## Implementation Decisions

### Port & Access
- Dedicated port for Cockpit, separate from opencode web UI
- Default port: 9090 (standard Cockpit port)
- Port configurable via `occ config set cockpit_port <port>`
- Uses same `bind_address` as opencode (no separate setting)
- Cockpit enabled by default when container runs
- `occ status` shows both opencode and Cockpit URLs
- New `occ cockpit` command opens Cockpit in browser
- Cockpit requires container to be running (lives inside container)

### Feature Scope
- Default to minimal plugin set: System overview + Terminal + Services
- Config option `cockpit_mode: minimal|full` (requires rebuild to change)
- Minimal is default, user opts into full
- Full mode includes everything Cockpit offers
- Container-only visibility (no host system info)
- Terminal defaults to opencode user but allows root access
- Full service control (start/stop/restart) for container services

### User Experience
- Both wizard and `occ start` output mention Cockpit availability
- Wizard prompts for Cockpit port during setup (default 9090)
- `occ cockpit` shows error with instructions if container not running
- `occ status` shows Cockpit health status
- `occ cockpit --help` provides command documentation
- `cockpit_enabled` config toggle for runtime disable
- Rebuild with `cockpit_enabled=false` removes entirely

### Authentication
- Same PAM users as opencode (users from `occ user add` work for Cockpit)
- All opencode users can access Cockpit (no admin role distinction)
- Independent sessions (no SSO between opencode and Cockpit)
- Honors `allow_unauthenticated_network` if set
- Uses same `rate_limit_*` settings as opencode
- HTTP allowed (like opencode), TLS termination handled externally
- Honors `trust_proxy` setting for X-Forwarded headers
- Uses same network exposure warning as opencode

### Claude's Discretion
- How Cockpit logs appear in `occ logs` (mixed or separate)
- Cockpit installation method and package selection
- systemd service configuration inside container

</decisions>

<specifics>
## Specific Ideas

- Cockpit should feel like a natural extension of the CLI, not a separate product
- Same credentials, same network settings, same security model
- Users who discover it via wizard or status output should be able to use it immediately

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-remote-administration-via-cockpit*
*Context gathered: 2026-01-22*
