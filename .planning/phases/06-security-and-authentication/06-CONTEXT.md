# Phase 6: Security and Authentication - Context

**Gathered:** 2026-01-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Securing the opencode web UI with authentication and controlling network exposure. This phase implements PAM-based authentication (opencode-cloud configures system users in the container, opencode authenticates against PAM), network binding controls (localhost by default, explicit opt-in for network exposure), and load balancer compatibility. Audit logging and user roles are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Network binding behavior
- Default binding: 127.0.0.1 only (localhost) — safest default
- Opt-in for network exposure: Config file only (`occ config set bind_address 0.0.0.0`) — must be deliberate, no CLI flag for temporary exposure
- Warning timing: Both on config set AND brief reminder on every start when network exposed
- Auth gate: Strong warning but allow starting with 0.0.0.0 and no users configured (user may have intentional use case)
- Explicit unauthenticated network: `allow_unauthenticated_network: true` with double opt-in (config setting + interactive Y/N confirmation on first start)
- Config key: `allow_unauthenticated_network` — explicit and self-documenting
- Status display: Show bind address + access URLs AND security badge (`[LOCAL ONLY]` or `[NETWORK EXPOSED]`)
- IPv4 addresses only for now, but allow for extensibility (interface names deferred)
- IPv6 support: Yes, accept IPv6 addresses (::1 for localhost, :: for all interfaces)
- IPv6 status URLs: Show both IPv4 and IPv6 access URLs
- Validation: Both on config set AND re-validate on start (in case config edited manually)
- Default port: 3000
- Port conflict: Error with clear message + offer to use next available port interactively
- Multi-bind: Deferred to future — single address:port only for now

### Auth credential flow (PAM-based)
- Architecture: opencode uses PAM, so opencode-cloud configures system users on the container
- User management: Both via setup wizard (first user) AND `occ user` commands for additional users
- Commands: Full set — `add`, `remove`, `list`, `passwd`, `enable`, `disable`
- Password input: Interactive prompt by default, `--generate` flag creates random password and displays it
- Default username: "opencode" — setup wizard prompts for password only
- Password policy: Accept any password but warn if weak
- User list display: Table with username, status (enabled/disabled), created date, last login (if available)
- User storage: Config mirrors container — tracks usernames (not passwords) for status/management
- User persistence on rebuild: Auto-recreate from config, prompt user about reusing stored hashed passwords, be very vocal about the process
- Remove confirmation: Required unless `--force` flag
- Last user protection: Block removal of last user — "Cannot remove last user. Add another user first or use --force."
- Disable mechanism: Lock account in PAM using standard `passwd -l` / `usermod -L`
- User roles: Deferred to future — all users equal for now
- Auth status display: Show full user list with enabled/disabled in `occ status`
- Password change: No current password verification required (CLI user has shell access anyway)
- Generated password format: Random alphanumeric (16-24 chars), using secure crypto primitives (document which APIs)
- Container state: Error immediately if container not running — "Container not running. Start with `occ start` first."
- Rebuild state: Block user commands with message — "Container is rebuilding. Please wait."
- User shell: /bin/bash
- Home directories: Yes, create /home/<username> for each user

### Load balancer integration
- Detection: Both trust proxy headers AND explicit config (`trust_proxy: true`) — be clear about this in wizard, config, and README
- Health endpoint auth: Always public — /health bypasses authentication for LB checks
- Health response: Simple 200 OK with "OK" body
- Readiness endpoint: Deferred to future — just /health for now
- SSL handling: LB terminates SSL — opencode-cloud always serves HTTP
- Proxy headers: Standard set only — X-Forwarded-For, X-Forwarded-Proto, X-Forwarded-Host
- Documentation: Multiple examples — AWS ALB, nginx, Cloudflare tunnel

### Security feedback to user
- First start without security: Block with setup prompt — "Security not configured. Run `occ setup` first."
- Warning verbosity: Detailed with action — multi-line explaining risk and how to fix
- Audit logging: Deferred to future
- Security status: Yes, prominent "Security" section in `occ status` with binding, auth, warnings
- Unauthenticated network confirmation: Interactive Y/N with clear warning
- Sensitive values in config show: Masked always (password_hash: ********)
- Dedicated security-check command: Deferred to future
- Auth failure messages: Generic error only — "Authentication failed" (no user enumeration)
- Rate limiting: Basic limiting — configurable attempts per minute per IP with increasing delays
- Rate limit config: Yes — `rate_limit_attempts`, `rate_limit_window_seconds`
- Rate limit exceeded: Increasing delay pattern (1s, 5s, 30s, 5min progressive)
- Warning formatting: Auto-detect — color if TTY, plain text if piped/redirected

### Claude's Discretion
- Exact warning message wording
- HTTPS redirect logic when behind proxy
- Rate limiting default values
- Specific increasing delay intervals
- Password strength detection heuristics

</decisions>

<specifics>
## Specific Ideas

- "Be very vocal about the process so that the user/developer understands what is happening, especially in terms of authentication and security" — transparency is key
- "Make any error messages or warnings make this super clear and documentation should be clear like the README and stuff" — documentation quality matters
- "Ensure we are using secure crypto primitives to generate [passwords] and make that clear to the user exactly how we are generating it and which APIs even" — security transparency
- opencode fork integrates PAM — this shapes the entire auth architecture
- No temporary exposure flags — deliberate config-only changes for security

</specifics>

<deferred>
## Deferred Ideas

- Interface name binding (eth0, en0) — future enhancement
- Multi-bind (multiple addresses/ports) — future enhancement
- User roles/permissions — future enhancement
- Audit logging of auth events — future enhancement
- Separate readiness endpoint (/ready) — future enhancement
- Dedicated security-check command — future enhancement

</deferred>

---

*Phase: 06-security-and-authentication*
*Context gathered: 2026-01-20*
