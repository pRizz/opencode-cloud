# Requirements: opencode-cloud

**Defined:** 2026-01-18
**Core Value:** Developers can access a persistent, secure opencode instance from anywhere without wrestling with Docker, service management, or cloud infrastructure details.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Installation

- [ ] **INST-01**: User can install via `npx opencode-cloud` or `cargo install opencode-cloud` (both compile from source; Rust 1.85+ required)
- [x] **INST-02**: Installation wizard guides user through initial setup
- [x] **INST-03**: Wizard asks for auth credentials (username/password)
- [x] **INST-04**: Wizard asks for port and hostname with sensible defaults
- [x] **INST-05**: Wizard allows user to defer API key configuration (can set in opencode later)
- [ ] **INST-06**: User can uninstall cleanly via `opencode-cloud uninstall`
- [ ] **INST-07**: Clear error messages with actionable guidance
- [ ] **INST-08**: Help documentation available via `--help` for all commands

### Service Lifecycle

- [ ] **LIFE-01**: User can start service via `opencode-cloud start`
- [ ] **LIFE-02**: User can stop service via `opencode-cloud stop`
- [ ] **LIFE-03**: User can restart service via `opencode-cloud restart`
- [ ] **LIFE-04**: User can check status via `opencode-cloud status`
- [ ] **LIFE-05**: User can view logs via `opencode-cloud logs`
- [ ] **LIFE-06**: User can update opencode to latest via `opencode-cloud update`
- [ ] **LIFE-07**: Health check endpoint available for monitoring (e.g., `/health`)

### Configuration

- [x] **CONF-01**: User can configure port for web UI
- [x] **CONF-02**: User can configure basic auth credentials
- [x] **CONF-03**: User can configure environment variables for opencode
- [ ] **CONF-04**: Configuration persisted in JSON file at platform-appropriate location
- [x] **CONF-05**: User can view/edit config via `opencode-cloud config`
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

- [ ] **CONS-01**: Single instance per host (one opencode-cloud per machine)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Multi-Instance

- **MULT-01**: Multiple sandboxed opencode environments on single host

### Cloud Config Sync

- **SYNC-01**: Backup opencode-cloud config to Google Drive
- **SYNC-02**: Backup opencode's own config to Google Drive
- **SYNC-03**: Optional backup of session history to Google Drive
- **SYNC-04**: Restore config from Google Drive on new deployment
- **SYNC-05**: Unique device naming to avoid conflicts
- **SYNC-06**: Optional replicated config across deployments (with stability warnings)
- **SYNC-07**: User-configurable versioned backups in Google Drive (keep N previous versions)

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
- **ADMN-02**: Separate auth layer for remote terminal (may differ from opencode web UI auth)

## v3 Requirements

Future vision. Tracked for planning purposes.

### Remote Admin (v3)

- **ADMN-03**: Basic remote desktop environment for sandboxed environment
- **ADMN-04**: Auth layer for remote desktop (may share or differ from terminal auth)

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
| INST-01 | Phase 1 | Complete |
| INST-02 | Phase 5 | Complete |
| INST-03 | Phase 5 | Complete |
| INST-04 | Phase 5 | Complete |
| INST-05 | Phase 5 | Complete |
| INST-06 | Phase 8 | Pending |
| INST-07 | Phase 8 | Pending |
| INST-08 | Phase 8 | Pending |
| LIFE-01 | Phase 3 | Complete |
| LIFE-02 | Phase 3 | Complete |
| LIFE-03 | Phase 3 | Complete |
| LIFE-04 | Phase 3 | Complete |
| LIFE-05 | Phase 3 | Complete |
| LIFE-06 | Phase 7 | Pending |
| LIFE-07 | Phase 7 | Pending |
| CONF-01 | Phase 5 | Complete |
| CONF-02 | Phase 5 | Complete |
| CONF-03 | Phase 5 | Complete |
| CONF-04 | Phase 1 | Complete |
| CONF-05 | Phase 5 | Complete |
| CONF-06 | Phase 7 | Pending |
| CONF-07 | Phase 1 | Complete |
| PERS-01 | Phase 4 | Complete |
| PERS-02 | Phase 2 | Complete |
| PERS-03 | Phase 2 | Complete |
| PERS-04 | Phase 2 | Complete |
| PERS-05 | Phase 4 | Complete |
| PERS-06 | Phase 4 | Complete |
| SECU-01 | Phase 6 | Pending |
| SECU-02 | Phase 6 | Pending |
| SECU-03 | Phase 6 | Pending |
| SECU-04 | Phase 6 | Pending |
| PLAT-01 | Phase 4 | Complete |
| PLAT-02 | Phase 4 | Complete |
| CONS-01 | Phase 1 | Complete |

**Coverage:**
- v1 requirements: 33 total
- Mapped to phases: 33
- Unmapped: 0

---
*Requirements defined: 2026-01-18*
*Last updated: 2026-01-20 (Phase 5 complete: INST-02, INST-03, INST-04, INST-05, CONF-01, CONF-02, CONF-03, CONF-05)*
