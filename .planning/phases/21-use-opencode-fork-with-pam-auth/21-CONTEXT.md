# Phase 21: Use opencode Fork with PAM Authentication - Context

**Gathered:** 2026-01-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Switching from mainline opencode (installed via `curl -fsSL https://opencode.ai/install | bash`) to the pRizz fork (https://github.com/pRizz/opencode) which implements PAM-based web authentication. This enables users created via `occ user add` to authenticate directly to the opencode web UI using the same container system users that Cockpit uses, providing unified authentication across both services.

Current state:
- Dockerfile installs opencode via official installer script (https://opencode.ai/install)
- opencode runs as systemd service with `opencode web --port 3000 --hostname 0.0.0.0`
- Phase 6 implemented PAM user management (`occ user add/remove/list/passwd/enable/disable`)
- Legacy `auth_username`/`auth_password` config fields exist but are deprecated (warnings shown in validation)
- Config tracks usernames in `config.users[]` array for persistence

Target state:
- Dockerfile installs opencode from pRizz/opencode fork (GitHub repository)
- opencode web UI authenticates against PAM (same users as Cockpit)
- Users created via `occ user add` can log into opencode web UI
- Legacy auth fields fully deprecated/removed
- Documentation updated to reflect PAM-based flow

</domain>

<decisions>
## Implementation Decisions

### Fork installation method
- **Installation approach**: Build from source following [docker-install-fork.md](https://github.com/pRizz/opencode/blob/dev/docs/docker-install-fork.md)
- **Installation method**: Clone repo, build with bun, copy binary to ~/.opencode/bin/
- **Version pinning**: Pin to specific commit `11277fd04fb7f6df4b6f188397dcb5275ef3a78f` for reproducibility
- **Build command**: `bun run packages/opencode/script/build.ts --single` (builds for current platform)
- **Binary location**: `/tmp/opencode/packages/opencode/dist/opencode-*/bin/opencode` → `/home/opencode/.opencode/bin/opencode`

### PAM integration verification
- **PAM integration**: ✅ Confirmed - fork implements PAM authentication via opencode-broker service
- **Documentation**: [pam-config.md](https://github.com/pRizz/opencode/blob/dev/docs/pam-config.md) provides complete setup guide
- **Components required**:
  1. PAM configuration file: `/etc/pam.d/opencode` (from `packages/opencode-broker/service/opencode.pam`)
  2. opencode-broker binary: Rust service that handles PAM auth (build from `packages/opencode-broker`)
  3. opencode-broker.service: systemd unit file (from `packages/opencode-broker/service/opencode-broker.service`)
  4. Unix socket: `/run/opencode/broker.sock` (created by broker)
- **Testing approach**: Test PAM auth works with users created via `occ user add`

### Dockerfile changes
- **Location**: `packages/core/src/docker/Dockerfile` (embedded in Rust code)
- **Current section**: Lines 420-433 (opencode installation)
- **Current systemd service**: Lines 444-468 (opencode.service unit file)
- **Changes needed**:
  1. **Replace opencode installation** (lines 420-433):
     - Replace installer script with git clone + build from source
     - Pin to commit `11277fd04fb7f6df4b6f188397dcb5275ef3a78f`
     - Use bun to build: `bun run packages/opencode/script/build.ts --single`
  2. **Build opencode-broker** (new section):
     - Build Rust binary from `packages/opencode-broker`
     - Install to `/usr/local/bin/opencode-broker`
     - Set setuid permissions: `chmod 4755` (or run as root via systemd)
  3. **Install PAM configuration** (new section):
     - Copy `packages/opencode-broker/service/opencode.pam` to `/etc/pam.d/opencode`
  4. **Add opencode-broker.service** (new section):
     - Copy service file from `packages/opencode-broker/service/opencode-broker.service`
     - Enable service: `systemctl enable opencode-broker`
  5. **Update opencode.service** (lines 444-468):
     - Add dependency: `After=opencode-broker.service`
     - Command remains the same: `opencode web --port 3000 --hostname 0.0.0.0`
  6. **Create opencode.json config** (new section):
     - Enable auth: `{"auth": {"enabled": true}}`
     - Place in `/home/opencode/.config/opencode/opencode.json` or similar

### Legacy auth field deprecation
- **Current state**: `auth_username` and `auth_password` fields exist in config schema
- **Current behavior**: Validation shows warnings when legacy fields are present
- **Migration path**: **No migration** - keep fields for backward compatibility but ignore them
- **Documentation**: Clearly document that legacy fields are deprecated and PAM users should be used instead
- **Wizard behavior**: Update wizard to create PAM users instead of setting legacy fields

### Documentation updates
- **README.md**: Update installation/authentication sections
- **CONTRIBUTING.md**: Update if Dockerfile changes affect contributors
- **Config schema docs**: Remove or deprecate legacy auth fields
- **User guide**: Document PAM-based authentication flow

### Systemd service compatibility
- **Current service**: Runs `opencode web --port 3000 --hostname 0.0.0.0` - command remains the same
- **PAM requirements**: opencode needs auth config enabled in `opencode.json`: `{"auth": {"enabled": true}}`
- **Additional service**: Need to add `opencode-broker.service` that runs before opencode service
- **Service dependencies**: opencode.service should `After=opencode-broker.service` to ensure broker is ready
- **User context**: Broker runs as root (or setuid), opencode service runs as `opencode` user
- **Port binding**: Currently binds to 0.0.0.0 in service, but container port binding uses `bind_address` config
- **Socket permissions**: Broker socket at `/run/opencode/broker.sock` with 0666 permissions (world-readable/writable, safe because PAM authenticates)

### Claude's Discretion
- Exact installation method for GitHub fork
- Version pinning strategy
- Legacy field migration approach
- Documentation wording
- Error messages for PAM auth failures

</decisions>

<specifics>
## Specific Ideas

- "The mainline opencode uses basic auth stored in config. The pRizz fork integrates with PAM for web authentication" - from ROADMAP.md
- "allowing users created via `occ user add` to authenticate directly to the opencode web UI using the same container system users that Cockpit uses" - unified auth is key benefit
- Phase 6 CONTEXT.md mentions: "opencode fork integrates PAM — this shapes the entire auth architecture"
- Phase 6 RESEARCH.md notes: "opencode OPENCODE_SERVER_PASSWORD | PAM login | User decision (forked opencode) | More robust, standard Linux auth"
- Current Dockerfile uses retry logic for installer (5 attempts) - may need similar for GitHub clone/install

</specifics>

<resolved_questions>
## Resolved Questions

1. **Fork installation method** ✅
   - **Answer**: Clone repo, build from source with bun, pin to commit `11277fd04fb7f6df4b6f188397dcb5275ef3a78f`
   - **Reference**: [docker-install-fork.md](https://github.com/pRizz/opencode/blob/dev/docs/docker-install-fork.md)

2. **PAM integration details** ✅
   - **Answer**: Fork implements PAM via opencode-broker service (Rust binary)
   - **Components**: PAM config file, broker binary, broker systemd service, Unix socket
   - **Reference**: [pam-config.md](https://github.com/pRizz/opencode/blob/dev/docs/pam-config.md)

3. **Legacy auth migration** ✅
   - **Answer**: No migration - keep fields for backward compatibility, ignore them, document deprecation clearly

4. **Systemd service compatibility** ✅
   - **Answer**: `opencode web` command works the same, but need to add opencode-broker.service
   - **Dependencies**: opencode.service should `After=opencode-broker.service`
   - **Config**: Need `opencode.json` with `{"auth": {"enabled": true}}`

5. **Testing strategy** ✅
   - **Answer**: Test end-to-end: create user via `occ user add`, verify can log into opencode web UI
   - **Verification**: Check broker socket exists, broker service running, PAM config installed

</resolved_questions>

<deferred>
## Deferred Ideas

- Multi-provider authentication (OAuth, LDAP, etc.) - PAM is sufficient for now
- User roles/permissions in opencode - all users equal for now
- Audit logging of opencode auth events - deferred to future

</deferred>

<implementation_details>
## Implementation Details

### opencode-broker Service
- **Location**: `packages/opencode-broker` (Rust project)
- **Build**: `cargo build --release` in broker directory
- **Binary**: `target/release/opencode-broker` → `/usr/local/bin/opencode-broker`
- **Permissions**: Setuid root (`chmod 4755`) or run as root via systemd
- **Socket**: Creates `/run/opencode/broker.sock` with 0666 permissions
- **Service file**: `packages/opencode-broker/service/opencode-broker.service`
- **Runtime directory**: `/run/opencode` (created by systemd RuntimeDirectory)

### PAM Configuration
- **File**: `packages/opencode-broker/service/opencode.pam`
- **Install location**: `/etc/pam.d/opencode`
- **Contents**: Standard UNIX auth (`pam_unix.so`) for auth and account
- **Optional**: 2FA support via `pam_google_authenticator.so` (deferred for now)

### opencode Configuration
- **Config file**: `~/.config/opencode/opencode.json` or similar
- **Minimal config**: `{"auth": {"enabled": true}}`
- **PAM service name**: Defaults to "opencode" (matches `/etc/pam.d/opencode`)
- **Socket path**: Defaults to `/run/opencode/broker.sock`

### Service Dependencies
```
opencode-broker.service (runs first, creates socket)
    ↓
opencode.service (depends on broker, connects to socket)
```

### Build Order in Dockerfile
1. Install opencode from fork (clone, build, install binary)
2. Build opencode-broker (cargo build, install binary, set permissions)
3. Install PAM config file
4. Create opencode-broker.service systemd unit
5. Create/update opencode.service (add After=opencode-broker.service)
6. Create opencode.json config file with auth enabled

</implementation_details>

---

*Phase: 21-use-opencode-fork-with-pam-auth*
*Context gathered: 2026-01-26*
