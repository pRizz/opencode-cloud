# Phase 7: Update and Maintenance - Context

**Gathered:** 2026-01-20
**Status:** Ready for planning

<domain>
## Phase Boundary

User can update opencode to latest version, monitor service health via endpoint, and get clear feedback when config is invalid. Covers `occ update` command, `/health` endpoint, and config validation on startup.

</domain>

<decisions>
## Implementation Decisions

### Update command behavior
- Update scope: Pull latest image AND recreate users from config.users array
- Downtime approach: Simple stop → update → start sequence (brief downtime expected)
- Rollback support: Tag previous image before update, offer `occ update --rollback`
- Progress feedback: Step-by-step status showing each phase (stopping, pulling, creating, starting)

### Health check endpoint
- Response content: Detailed JSON with container state, uptime, memory, version
- Authentication: No auth required (public endpoint for load balancers/monitoring)
- Hosting: Proxy through opencode (configure opencode to serve /health)
- HTTP status codes: 200 for healthy, 503 for unhealthy (standard load balancer pattern)

### Config validation feedback
- Validation timing: Validate on `config set` AND on start
- Error presentation: Single error at a time, stop at first error
- Fix guidance: Include exact command to fix the error
- Warning handling: Configurable to treat warnings as errors, default shows warnings but continues

### Update notifications
- Auto-check: Periodic background check (not blocking)
- Check frequency: Configurable, weekly by default
- Notification style: Subtle one-liner at end of command output
- Opt-out: Config option `update_notifications` to disable

### Claude's Discretion
- Exact health check JSON schema
- How to detect opencode version for comparison
- Background check implementation (file timestamp vs cron-like)
- Warning vs error classification for config issues

</decisions>

<specifics>
## Specific Ideas

- Rollback should be simple: `occ update --rollback` restores previous tagged image
- Health endpoint should work with AWS ALB health checks out of the box
- Update notifications should not slow down CLI commands (async/cached)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-update-and-maintenance*
*Context gathered: 2026-01-20*
