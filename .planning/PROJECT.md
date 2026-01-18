# opencode-cloud

## What This Is

A production-ready toolkit for deploying opencode (the open-source AI coding agent) as a persistent cloud service. Provides Docker container configuration, a cross-platform CLI (`opencode-cloud` / `occ`) installable via npm or cargo, an interactive setup wizard, and service management commands — making it easy to run opencode's web UI from anywhere in the world with proper security and resilience.

## Core Value

Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Docker container configuration that runs opencode web UI
- [ ] CLI installable via `npx opencode-cloud` or `cargo install opencode-cloud`
- [ ] CLI command: `opencode-cloud` with alias `occ`
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

- **Architecture**: Rust core library with NAPI-RS bindings for Node.js (single codebase, two distribution channels)
- **Monorepo**: `packages/core` (Rust), `packages/cli-rust` (Rust CLI), `packages/cli-node` (Node CLI wrapper)
- **Cross-platform**: Must support Linux and macOS (Windows deferred to v2)
- **Custom Docker image**: We supply our own Ubuntu-based Dockerfile for complete control; user-configurable base image deferred to future
- **Config format**: JSONC (JSON with comments) for persistent configuration
- **Config location**: XDG-compliant paths (`~/.config/opencode-cloud/config.json` on Linux/macOS)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Project name: opencode-cloud | Cleaner than opencode-cloud-service, CLI command `occ` is memorable | — Pending |
| Rust core + NAPI-RS bindings | Single codebase serves both npm and cargo users; npm gets prebuilt binaries | — Pending |
| Custom Ubuntu-based Dockerfile | Complete control over sandbox environment; can respond to upstream issues | — Pending |
| Use opencode's built-in basic auth | Avoid reinventing authentication, leverage upstream config options | — Pending |
| Auth credentials stored on host | Credentials in host config file (platform-appropriate path), passed to opencode process | — Pending |
| Monorepo structure | `packages/core`, `packages/cli-rust`, `packages/cli-node` with pnpm + Cargo workspaces | — Pending |
| JSONC for config | JSON with comments for user-friendliness, easy inspection | — Pending |
| XDG paths on macOS | Use `~/.config/` instead of `~/Library/` for consistency across platforms | — Pending |
| Google Drive sync deferred to v2 | Reduces MVP scope significantly | — Pending |
| Windows support deferred to v2 | Focus on Linux/macOS for v1 | — Pending |
| Separate auth for remote admin (v2/v3) | Remote terminal/desktop may need different credentials than opencode web UI | — Pending |

---
*Last updated: 2026-01-18 after Phase 1 context gathering*
