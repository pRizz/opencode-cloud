# Phase 3: Service Lifecycle Commands - Context

**Gathered:** 2026-01-19
**Status:** Ready for planning

<domain>
## Phase Boundary

User can control the service through intuitive CLI commands: start, stop, restart, status, and logs. This phase builds the user-facing commands that wrap Docker operations from Phase 2. Authentication, platform service registration, and configuration wizards are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Command output
- Spinner with status text during operations ("Starting container...", "Waiting for health check...")
- Elapsed time shown during long operations (e.g., "Starting container... (12s)")
- On successful start: show URL, container ID, port mapping, volume paths, plus prompt to open in browser
- `--open` flag auto-opens browser; config option to always open by default
- No confirmation required for stop/restart (config option to enable confirmation if desired)
- Idempotent behavior: `start` when running prints status and exits 0; `stop` when stopped prints status and exits 0
- `-q` quiet flag for scripting: suppresses spinner and info
- `-v` verbose flag for extra debugging output
- Colors enabled by default when TTY detected, plain when piped
- `--no-color` flag and `NO_COLOR` env var both supported to disable colors
- 30-second graceful shutdown timeout before force-kill
- No `--json` flag for v1 (keep it simple)

### Status display
- Key-value line format (not boxed/table)
- Shows: State (colored), URL, Container name + ID, Image version, Uptime (duration + since timestamp), Port mapping, Health check state, Config file path
- When stopped: shows state, last run time (if known), and hint to start
- Default command when config exists (wizard has been run); otherwise show help
- Rich dashboard (CPU/memory usage) deferred to post-MVP, behind a flag

### Logs behavior
- Default: follow mode (like `docker logs -f`)
- Shows last 50 lines before following
- `-n` / `--lines` flag to specify line count
- `--no-follow` flag for one-shot dump and exit
- `--timestamps` flag to prefix lines with timestamps (off by default)
- `--grep` flag for filtering lines
- `-q` quiet flag suppresses status messages, just raw log lines
- Color-coded by log level (ERROR red, WARN yellow, etc.) when TTY detected
- Shows logs from stopped container if available
- Combined stdout/stderr (Docker default)
- Exits with message when container stops during follow
- No `--since` time filter for v1

### Error handling
- Actionable error messages with specific guidance (e.g., "Add user to docker group: `sudo usermod -aG docker $USER`")
- Include documentation link in common errors (GitHub troubleshooting section)
- Auto-build image if not found (no prompt)
- Auto-suggest next available port if configured port in use
- Container crash on start: show last 20 log lines automatically
- Timeout after 30 seconds waiting for startup, show message suggesting to check status
- Verbose error context by default
- Proper exit codes: 0 success, 1 error
- Proper streams: errors to stderr, normal output to stdout

### Claude's Discretion
- Quiet mode output for successful start (URL only vs nothing)
- Status -q behavior (text vs exit code)
- Multiple container conflict resolution
- Specific spinner animation style
- Exact color choices for states/log levels

</decisions>

<specifics>
## Specific Ideas

- Commands should feel like standard Docker CLI ergonomics
- Verbose errors help users self-diagnose without needing to file issues
- Update availability indicator deferred to next milestone

</specifics>

<deferred>
## Deferred Ideas

- Rich dashboard with CPU/memory usage (post-MVP, behind flag)
- `--json` flag for machine-readable output
- `--since` time filter for logs
- Update available indicator in status

</deferred>

---

*Phase: 03-service-lifecycle-commands*
*Context gathered: 2026-01-19*
