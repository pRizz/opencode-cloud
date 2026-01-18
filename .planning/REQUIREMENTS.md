# Requirements: opencode-cloud-service

**Defined:** 2026-01-18
**Core Value:** Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Installation

- [ ] **INST-01**: User can install via `npx opencode-cloud-service` or `cargo install opencode-cloud-service`
- [ ] **INST-02**: Installation wizard guides user through initial setup
- [ ] **INST-03**: Wizard asks for auth credentials (username/password)
- [ ] **INST-04**: Wizard asks for port and hostname with sensible defaults
- [ ] **INST-05**: Wizard allows user to defer API key configuration (can set in opencode later)
- [ ] **INST-06**: User can uninstall cleanly via `opencode-cloud-service uninstall`
- [ ] **INST-07**: Clear error messages with actionable guidance
- [ ] **INST-08**: Help documentation available via `--help` for all commands

### Service Lifecycle

- [ ] **LIFE-01**: User can start service via `opencode-cloud-service start`
- [ ] **LIFE-02**: User can stop service via `opencode-cloud-service stop`
- [ ] **LIFE-03**: User can restart service via `opencode-cloud-service restart`
- [ ] **LIFE-04**: User can check status via `opencode-cloud-service status`
- [ ] **LIFE-05**: User can view logs via `opencode-cloud-service logs`
- [ ] **LIFE-06**: User can update opencode to latest via `opencode-cloud-service update`
- [ ] **LIFE-07**: Health check endpoint available for monitoring (e.g., `/health`)

### Configuration

- [ ] **CONF-01**: User can configure port for web UI
- [ ] **CONF-02**: User can configure basic auth credentials
- [ ] **CONF-03**: User can configure environment variables for opencode
- [ ] **CONF-04**: Configuration persisted in JSON file at platform-appropriate location
- [ ] **CONF-05**: User can view/edit config via `opencode-cloud-service config`
- [ ] **CONF-06**: Config validated on service startup with clear error messages
- [ ] **CONF-07**: Config format documented with JSON schema

### Persistence & Reliability

- [ ] **PERS-01**: Service survives host reboots (registered with systemd/launchd)
- [ ] **PERS-02**: AI session history persisted across restarts
- [ ] **PERS-03**: Project files persisted across restarts
- [ ] **PERS-04**: Configuration persisted across restarts
- [ ] **PERS-05**: Service auto-restarts on crash
- [ ] **PERS-06**: User can configure restart policies (retry count, delay)

### Security

- [ ] **SECU-01**: Basic authentication required to access web UI
- [ ] **SECU-02**: Service binds to localhost by default
- [ ] **SECU-03**: Explicit opt-in required for network exposure (0.0.0.0 binding)
- [ ] **SECU-04**: Designed to work behind load balancer with SSL termination

### Platform Support

- [ ] **PLAT-01**: Linux support with systemd service integration
- [ ] **PLAT-02**: macOS support with launchd service integration

### Constraints

- [ ] **CONS-01**: Single instance per host (one opencode-cloud-service per machine)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Multi-Instance

- **MULT-01**: Multiple sandboxed opencode environments on single host

### Cloud Config Sync

- **SYNC-01**: Backup opencode-cloud-service config to Google Drive
- **SYNC-02**: Backup opencode's own config to Google Drive
- **SYNC-03**: Optional backup of session history to Google Drive
- **SYNC-04**: Restore config from Google Drive on new deployment
- **SYNC-05**: Unique device naming to avoid conflicts
- **SYNC-06**: Optional replicated config across deployments (with stability warnings)

### Additional Platforms

- **PLAT-03**: Windows support with Windows services integration

### CLI Enhancements

- **CLI-01**: Non-interactive mode for scripting/CI
- **CLI-02**: Quiet mode (minimal output)
- **CLI-03**: Verbose mode (detailed output)

### Advanced Security

- **SECU-05**: Optional TLS configuration for deployments without load balancer

### Remote Admin (v2)

- **ADMN-01**: Remote terminal access to sandboxed environment via web interface

## v3 Requirements

Future vision. Tracked for planning purposes.

### Remote Admin (v3)

- **ADMN-02**: Basic remote desktop environment for sandboxed environment

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Full PaaS platform | Coolify/Dokploy exist; we're single-purpose |
| Web-based config UI | CLI is sufficient for target audience |
| Kubernetes support | Scope creep; native services are simpler |
| Plugin system | Premature abstraction |
| Multi-node clustering | Enterprise scope; single-host focus |
| OAuth/SSO integration | Basic auth sufficient for v1; upstream opencode may add later |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| INST-01 | TBD | Pending |
| INST-02 | TBD | Pending |
| INST-03 | TBD | Pending |
| INST-04 | TBD | Pending |
| INST-05 | TBD | Pending |
| INST-06 | TBD | Pending |
| INST-07 | TBD | Pending |
| INST-08 | TBD | Pending |
| LIFE-01 | TBD | Pending |
| LIFE-02 | TBD | Pending |
| LIFE-03 | TBD | Pending |
| LIFE-04 | TBD | Pending |
| LIFE-05 | TBD | Pending |
| LIFE-06 | TBD | Pending |
| LIFE-07 | TBD | Pending |
| CONF-01 | TBD | Pending |
| CONF-02 | TBD | Pending |
| CONF-03 | TBD | Pending |
| CONF-04 | TBD | Pending |
| CONF-05 | TBD | Pending |
| CONF-06 | TBD | Pending |
| CONF-07 | TBD | Pending |
| PERS-01 | TBD | Pending |
| PERS-02 | TBD | Pending |
| PERS-03 | TBD | Pending |
| PERS-04 | TBD | Pending |
| PERS-05 | TBD | Pending |
| PERS-06 | TBD | Pending |
| SECU-01 | TBD | Pending |
| SECU-02 | TBD | Pending |
| SECU-03 | TBD | Pending |
| SECU-04 | TBD | Pending |
| PLAT-01 | TBD | Pending |
| PLAT-02 | TBD | Pending |
| CONS-01 | TBD | Pending |

**Coverage:**
- v1 requirements: 33 total
- Mapped to phases: 0
- Unmapped: 33 (pending roadmap creation)

---
*Requirements defined: 2026-01-18*
*Last updated: 2026-01-18 after initial definition*
