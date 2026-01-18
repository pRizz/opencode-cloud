# opencode-cloud-service

## What This Is

A production-ready toolkit for deploying opencode (the open-source AI coding agent) as a persistent cloud service. Provides Docker container configuration, cross-platform CLI installers (npm and cargo), an interactive setup wizard, and service management commands — making it easy to run opencode's web UI from anywhere in the world with proper security and resilience.

## Core Value

Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Docker container configuration that runs opencode web UI
- [ ] npm/npx installer with interactive setup wizard
- [ ] cargo installer with interactive setup wizard
- [ ] Cross-platform service installation (Linux systemd, macOS launchd, Windows services)
- [ ] CLI commands: start, stop, status, logs, config, update
- [ ] Authentication via opencode's built-in basic auth
- [ ] Optional TLS configuration (for deployments not behind a load balancer)
- [ ] Designed to work behind AWS ALB/ELB with SSL termination
- [ ] Full persistence: session history, project files, configuration
- [ ] Auto-restart on crash with configurable retry policies
- [ ] Persistent JSON config file (inspectable, version-controllable)
- [ ] Reasonable defaults for ports, hostname, and other settings
- [ ] Moderate verbosity by default with quiet/verbose options

### Out of Scope

- Google Drive config backup/sync — v2 feature, not MVP
- Cross-deployment config sharing — v2 feature
- Mobile app — web UI accessed from mobile browser is sufficient
- Custom opencode fork — uses upstream opencode as-is

## Context

**Upstream project:** [opencode by anomalyco](https://github.com/anomalyco/opencode) — an open-source AI coding agent written in TypeScript. Has official Docker images at `ghcr.io/anomalyco/opencode:latest`. Web UI available via `opencode web` command.

**Known upstream issues:**
- [Remote access WebSocket bug](https://github.com/anomalyco/opencode/issues/5844) — workaround is passing URL explicitly via query parameter (`?url=http://IP:PORT`)
- opencode has built-in basic auth configuration we can leverage

**Related projects:**
- [opencode Portal](https://github.com/hosenur/portal) — third-party mobile-first UI, recommends Tailscale for secure access

**Target deployment:** EC2 or similar VPS, typically behind a load balancer handling TLS termination.

## Constraints

- **Monorepo**: npm and cargo packages in the same repository, sharing Docker configuration
- **Cross-platform**: Must support Linux, macOS, and Windows (within reason)
- **Upstream dependency**: Relies on opencode Docker images and behavior; must handle upstream changes gracefully
- **Config format**: JSON for persistent configuration

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Use opencode's built-in basic auth | Avoid reinventing authentication, leverage upstream | — Pending |
| Monorepo for npm + cargo | Shared Docker config, simpler maintenance | — Pending |
| JSON for config | Universal parsing, easy inspection | — Pending |
| Google Drive sync deferred to v2 | Reduces MVP scope significantly | — Pending |

---
*Last updated: 2026-01-18 after initialization*
