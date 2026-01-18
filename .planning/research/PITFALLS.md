# Domain Pitfalls

**Domain:** Cross-platform cloud service installer (npm + Rust CLI, Docker management, service installation)
**Researched:** 2026-01-18
**Confidence:** MEDIUM (verified via multiple sources, some platform-specific details from official docs)

---

## Critical Pitfalls

Mistakes that cause rewrites, security incidents, or major user frustration.

### Pitfall 1: Docker Permission Mismatch Between Host and Container

**What goes wrong:** Files created by Docker containers have different ownership than the host user, causing "Permission Denied" errors when users try to access config files or logs outside the container. Users run `sudo` to fix it, which cascades into more permission problems.

**Why it happens:** Docker containers typically run as root or a non-root user (like `node` or `www-data`), but the host filesystem has different UID/GID ownership. When mounting volumes, these don't automatically match.

**Consequences:**
- Users cannot edit config files created by the container
- Container cannot write to directories created by the user
- Escalating `sudo` usage corrupts permissions further
- Support tickets flood in from frustrated users

**Prevention:**
1. Use named Docker volumes instead of bind mounts where possible (Docker manages ownership)
2. If bind mounts are required, match UID/GID: `docker run -u $(id -u):$(id -g)`
3. Create directories with proper permissions in Dockerfile: `RUN mkdir /data && chown node:node /data`
4. Document the permission model clearly in installation docs
5. Provide a `--fix-permissions` command in your CLI that runs appropriate `chown` commands

**Detection:** Test installation as non-root user on fresh system. Check if user can edit all created files without `sudo`.

**Phase mapping:** Address in Phase 1 (Docker foundation) - this is a day-one blocker.

**Sources:**
- [Docker Permission Guide](https://eastondev.com/blog/en/posts/dev/20251217-docker-mount-permissions-guide/)
- [Docker CLI Issue #3202](https://github.com/docker/cli/issues/3202)

---

### Pitfall 2: Config File Location Inconsistency Across Platforms

**What goes wrong:** Tool stores configs in different locations on different platforms without following conventions. Users can't find their configs, can't back them up, and can't share dotfiles across machines.

**Why it happens:** Developers hardcode paths or use naive `~/.toolname` approach, ignoring XDG spec on Linux, `~/Library/Application Support` on macOS, and `%APPDATA%` on Windows.

**Consequences:**
- User home directories littered with dotfiles
- Config sharing across machines fails
- Users can't control config location via environment variables
- Platform-specific bugs from path handling differences

**Prevention:**
1. Follow XDG Base Directory spec on Linux: `$XDG_CONFIG_HOME` (default `~/.config/toolname`)
2. On macOS for CLI tools: Follow XDG spec (not `~/Library/Application Support` - that's for GUI apps)
3. On Windows: Use `%APPDATA%\toolname`
4. Allow environment variable override: `TOOLNAME_CONFIG_DIR`
5. Use established libraries: Rust's `dirs` crate, Node's `env-paths`
6. Document where configs live in `--help` and README

**Detection:** Run `find ~ -name "*toolname*" -type d` after installation - configs should be in predictable locations only.

**Phase mapping:** Address in Phase 1 (CLI foundation) - architectural decision that's hard to change later.

**Sources:**
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [XDG on macOS debate](https://github.com/nushell/nushell/issues/15024)
- [macOS dotfiles](https://becca.ooo/blog/macos-dotfiles/)

---

### Pitfall 3: Exposing Web UI Without Authentication

**What goes wrong:** Users deploy the service with web UI exposed to the internet, assuming "it's just for me." Attackers find it via Shodan/Censys and gain access to the AI coding agent, which can execute arbitrary code.

**Why it happens:**
- Default configs bind to `0.0.0.0` instead of `localhost`
- No authentication enabled by default
- Users don't understand network exposure implications
- "Security later" mindset during development

**Consequences:**
- Complete system compromise (AI agent can run code)
- Credential theft
- Data exfiltration
- Cryptomining abuse
- Legal liability for the project

**Prevention:**
1. Default to `localhost` binding - require explicit opt-in for network exposure
2. Require authentication setup before allowing non-localhost access
3. Implement authentication from day one (even basic auth is better than nothing)
4. Show prominent warning during setup if exposing to network without auth
5. Provide built-in HTTPS via Let's Encrypt integration or reverse proxy guidance
6. Include security headers: HSTS, CSP, X-Frame-Options
7. Rate limit all endpoints
8. Add `--expose-warning` flag that must be explicitly set for network binding

**Detection:** Run `netstat -tlnp` after installation - service should only listen on 127.0.0.1 by default.

**Phase mapping:** Address in Phase 2 (security foundation) - MUST be complete before any public release.

**Sources:**
- [OWASP Top 10 2025](https://www.ateamsoftsolutions.com/web-application-security-checklist-2025-complete-owasp-top-10-implementation-guide-for-ctos/)
- [Web Application Security Checklist](https://www.stackhawk.com/blog/web-application-security-checklist-10-improvements/)

---

### Pitfall 4: systemd/launchd Service Installation Permission Errors

**What goes wrong:** Service installation works for some users but fails for others with cryptic permission errors. Users end up running the service as root, which creates security vulnerabilities.

**Why it happens:**

**Linux (systemd):**
- Confusing distinction between system services (`/etc/systemd/system/`) and user services (`~/.config/systemd/user/`)
- User services require `loginctl enable-linger` to run without active session
- Forgetting `systemctl daemon-reload` after changes
- Not using absolute paths in service files

**macOS (launchd):**
- Loading LaunchAgent with `sudo` makes it a root process
- Incorrect file permissions on plist files (must be owned by user or root, no group/world write)
- Disabled state in override database persists across reinstalls

**Windows:**
- NSSM paths with spaces cause security vulnerabilities
- Local user account configuration bugs
- Service throttling if startup takes >1500ms

**Consequences:**
- Service fails to start with unhelpful error messages
- Users run as root to "fix" it, creating security holes
- Services don't survive reboots
- Hours of debugging platform-specific issues

**Prevention:**
1. For user-level services (recommended):
   - Linux: Install to `~/.config/systemd/user/`, enable linger, use `--user` flag
   - macOS: Install to `~/Library/LaunchAgents/`, never use `sudo` to load
   - Windows: Use user-level scheduled tasks or per-user NSSM
2. Validate all paths are absolute before writing service files
3. Check and fix file permissions before loading services
4. Test service installation on fresh user account (not admin)
5. Provide clear error messages: "Service failed because X, fix by doing Y"
6. Include `install --diagnose` command that checks common issues

**Detection:** Test on fresh user account without admin privileges. Test service survives reboot.

**Phase mapping:** Address in Phase 3 (service management) - requires careful platform abstraction.

**Sources:**
- [systemd/User ArchWiki](https://wiki.archlinux.org/title/Systemd/User)
- [Common Systemd Configuration Errors](https://devops.aibit.im/article/systemd-configuration-errors-fixes)
- [launchd Tutorial](https://www.launchd.info/)
- [NSSM Documentation](https://nssm.cc/)

---

### Pitfall 5: npm Global Install Permission Hell

**What goes wrong:** `npm install -g` fails with EACCES permission errors. Users run `sudo npm install -g`, which corrupts npm permissions further. Subsequent installs fail even harder.

**Why it happens:**
- Node.js installed via system package manager puts npm global dir in `/usr/local/lib/node_modules`
- Users don't have write access to system directories
- `sudo npm install` creates files owned by root in user directories (`~/.npm`)
- Native modules requiring compilation need build tools users don't have

**Consequences:**
- Installation fails for majority of users
- Users who "fix" with sudo have worse problems
- Support burden is enormous
- Users give up and don't use the tool

**Prevention:**
1. Don't require global installation - support `npx toolname` for first-class usage
2. If global install needed, detect permission issues and provide clear guidance
3. Recommend nvm/fnm for Node.js installation (avoids permission issues)
4. Check for required build tools (Python, C++ compiler) before attempting native module compilation
5. Prefer pure JS dependencies over native modules when possible
6. Pre-build native binaries for major platforms if native modules required
7. Provide alternative installation methods: direct binary download, Docker, Homebrew

**Detection:** Test `npm install -g` on fresh system without running as root.

**Phase mapping:** Address in Phase 1 (installation) - this is the first thing users encounter.

**Sources:**
- [node-gyp Troubleshooting](https://blog.openreplay.com/node-gyp-troubleshooting-guide-fix-common-installation-build-errors/)
- [npm Permission Issues](https://github.com/newrelic/node-native-metrics/issues/126)

---

## Moderate Pitfalls

Mistakes that cause delays, technical debt, or user friction.

### Pitfall 6: Incomplete Uninstaller Leaves Orphaned Files

**What goes wrong:** Uninstalling the tool leaves config files, service files, Docker volumes, and registry entries scattered across the system. Reinstallation behaves unexpectedly due to stale state.

**Why it happens:**
- Developers focus on installation, uninstall is afterthought
- No tracking of what files were created during install
- Platform-specific cleanup is complex
- Fear of deleting user data (configs)

**Prevention:**
1. Track all created files/services in a manifest during installation
2. Provide `uninstall` command that reads manifest and removes everything
3. Ask user about config files: "Remove configs? [y/N]"
4. Clean up:
   - Service files (systemd units, launchd plists, Windows services)
   - Docker containers and volumes (with confirmation)
   - Config directories
   - Cache directories
   - Log files
5. Verify cleanup with `uninstall --verify` command

**Detection:** Run uninstall, then `find / -name "*toolname*"` should return nothing (except maybe backup configs).

**Phase mapping:** Address in Phase 3 (service management) alongside installation.

**Sources:**
- [Remove Leftover Files Guide](https://www.easeus.com/pc-transfer/remove-leftover-files-after-uninstalling-software.html)
- [Homebrew Uninstall](https://www.itech4mac.net/2025/11/how-to-completely-uninstall-homebrew-from-macos-in-2025-2026-no-leftover-files/)

---

### Pitfall 7: Interactive CLI Wizard Frustrates Power Users

**What goes wrong:** Every operation requires stepping through an interactive wizard. Power users who know exactly what they want can't script the tool or run it in CI/CD.

**Why it happens:**
- Developers prioritize beginner experience
- Interactive mode is easier to implement than comprehensive flags
- No consideration for automation use cases

**Consequences:**
- Tool unusable in scripts/CI
- Power users avoid the tool
- No way to reproduce exact configuration

**Prevention:**
1. Every interactive prompt must have a corresponding CLI flag
2. Support `--yes` or `--non-interactive` flag to accept defaults
3. Support config file for complex setups: `--config setup.yaml`
4. Silent mode for CI: `--quiet` suppresses all non-error output
5. Interactive mode should demonstrate the equivalent non-interactive command
6. Allow re-running previous step to modify choices

**Detection:** Can entire setup be run non-interactively? Can it be scripted?

**Phase mapping:** Address from Phase 1 - build non-interactive first, add interactive as sugar.

**Sources:**
- [CLI UX Patterns](https://lucasfcosta.com/2022/06/01/ux-patterns-cli-tools.html)
- [Wizard UX Pattern](https://www.eleken.co/blog-posts/wizard-ui-pattern-explained)

---

### Pitfall 8: Upstream opencode Breaking Changes Break Users

**What goes wrong:** Upstream opencode releases new version with breaking changes. Your tool auto-updates or pins to `latest`, suddenly breaking all user installations.

**Why it happens:**
- Trusting semver to prevent breaking changes (it doesn't)
- Using `latest` tag in Docker images
- No testing against upstream releases
- No migration path for breaking changes

**Consequences:**
- Users wake up to broken installations
- No clear rollback path
- Support tickets spike
- Trust erodes

**Prevention:**
1. Pin specific versions of upstream dependencies (not `latest`, not `^major`)
2. Use lock files (package-lock.json, Cargo.lock) and commit them
3. Wait 14 days before adopting new upstream releases (let others find bugs)
4. Maintain CI that tests against upstream releases
5. Document breaking changes and migration paths in CHANGELOG
6. Support `--use-version X.Y.Z` flag to override pinned version
7. Consider maintaining compatibility shims for upstream API changes

**Detection:** Does CI fail if upstream releases breaking change? Is there a process for version updates?

**Phase mapping:** Address in Phase 1 (dependency management) and ongoing maintenance.

**Sources:**
- [Pinning Is Futile (Research Paper)](https://arxiv.org/html/2502.06662v1)
- [Renovate Upgrade Best Practices](https://docs.renovatebot.com/upgrade-best-practices/)
- [Why Pinning Still Matters](https://corner.buka.sh/the-myth-of-stability-in-semantic-versioning-and-why-pinning-versions-still-matters/)

---

### Pitfall 9: Cross-Platform Path Handling Bugs

**What goes wrong:** Paths work on developer's Mac but fail on Windows due to backslashes, drive letters, path length limits, or case sensitivity differences.

**Why it happens:**
- Testing only on one platform
- String concatenation instead of proper path APIs
- Ignoring Windows long path limitation (260 chars)
- Case-sensitive vs case-insensitive filesystem assumptions

**Consequences:**
- Silent failures or cryptic errors on Windows
- Files created in wrong locations
- Path traversal vulnerabilities

**Prevention:**
1. Use path libraries: Node's `path.join()`, Rust's `std::path::Path`
2. Never use string concatenation for paths
3. On Windows, handle extended-length paths (>260 chars) with `\\?\` prefix
4. Test on all three platforms in CI (or at minimum, test path handling)
5. Use forward slashes in configs (most tools handle this cross-platform)
6. Normalize paths before comparison
7. Document any platform-specific path requirements

**Detection:** CI tests on Windows, Linux, and macOS. Test with paths containing spaces and special characters.

**Phase mapping:** Address in Phase 1 (CLI foundation) - use correct patterns from the start.

**Sources:**
- [Cross-Platform File System Abstractions](https://github.com/rust-cli/team/issues/10)
- [Building Cross-Platform Tools in Rust](https://codezup.com/building-cross-platform-tools-rust-guide-windows-macos-linux/)

---

### Pitfall 10: Reverse Proxy Misconfiguration Exposes Admin Interfaces

**What goes wrong:** Users set up Traefik/Nginx/Caddy reverse proxy but misconfigure it, exposing admin interfaces without authentication or routing to wrong backends.

**Why it happens:**
- Reverse proxy configuration is complex
- Each proxy has different syntax
- Users copy-paste configs without understanding
- Default Traefik dashboard has no authentication

**Consequences:**
- Admin interfaces exposed to internet
- Authentication bypassed via direct IP access
- Wrong services exposed on wrong domains

**Prevention:**
1. Provide tested, copy-paste configs for major proxies (Traefik, Nginx, Caddy)
2. Default configs should include authentication
3. Document what each config line does
4. Provide `setup proxy` command that generates correct config
5. Warn if service is accessed without going through proxy
6. Check common misconfigurations in health endpoint

**Detection:** Test accessing service directly by IP vs through proxy. Test accessing admin interfaces.

**Phase mapping:** Address in Phase 4 (cloud deployment) - provide secure defaults.

**Sources:**
- [Reverse Proxy Comparison 2025](https://medium.com/@thomas.byern/npm-traefik-or-caddy-how-to-pick-the-reverse-proxy-youll-still-like-in-6-months-1e1101815e07)
- [Traefik vs Nginx](https://blog.lrvt.de/nginx-proxy-manager-versus-traefik/)

---

## Minor Pitfalls

Annoyances that are fixable but reduce polish.

### Pitfall 11: Missing Prerequisite Checks

**What goes wrong:** Installation proceeds, then fails halfway through because Docker isn't installed or wrong version is present.

**Prevention:**
1. Check all prerequisites before starting installation
2. Provide specific fix instructions: "Docker not found. Install with: `brew install docker`"
3. Check version requirements, not just presence
4. Fail fast with clear message, not cryptic error later

**Phase mapping:** Phase 1 (installation)

---

### Pitfall 12: No Upgrade Path

**What goes wrong:** Users must uninstall and reinstall to upgrade. Configs are lost. Services must be manually reconfigured.

**Prevention:**
1. Provide `upgrade` command that preserves configs
2. Migrate configs between versions automatically
3. Backup before upgrade, restore on failure
4. Support rollback: `upgrade --rollback`

**Phase mapping:** Phase 3 (service management)

---

### Pitfall 13: Logs Hard to Find or Absent

**What goes wrong:** Something fails, user has no idea where to look for logs or what went wrong.

**Prevention:**
1. Consistent log location across platforms (in config dir)
2. `logs` command to show recent logs
3. `--verbose` flag for detailed output
4. Include timestamp and log level in all output
5. Log to file by default, not just stdout

**Phase mapping:** Phase 1 (CLI foundation)

---

### Pitfall 14: Docker Compose Override Conflicts

**What goes wrong:** Users customize docker-compose.yml, then upgrade overwrites their changes.

**Prevention:**
1. Use docker-compose.override.yml pattern for user customizations
2. Never modify user's override file during upgrade
3. Document the override pattern prominently

**Phase mapping:** Phase 2 (Docker management)

---

### Pitfall 15: Timeout Too Short for Slow Operations

**What goes wrong:** Docker image pull or first build times out, leaving system in partial state.

**Prevention:**
1. Show progress for long operations
2. Use generous timeouts (or none) for network operations
3. Make operations idempotent so retry works
4. Provide `--timeout` flag for slow networks

**Phase mapping:** Phase 2 (Docker management)

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Installation | npm permission hell (#5) | Support npx, detect issues early |
| Installation | Missing prerequisite checks (#11) | Check before proceeding |
| CLI Foundation | Config file locations (#2) | Follow XDG from day one |
| CLI Foundation | Path handling bugs (#9) | Use path libraries, test cross-platform |
| CLI Foundation | Wizard annoys power users (#7) | Build non-interactive first |
| Docker Management | Permission mismatch (#1) | Named volumes, UID/GID matching |
| Docker Management | Compose override conflicts (#14) | Use override pattern |
| Service Management | systemd/launchd errors (#4) | Abstract platform differences |
| Service Management | Incomplete uninstaller (#6) | Track created files |
| Security | Web UI without auth (#3) | Default to localhost, require auth |
| Security | Reverse proxy misconfiguration (#10) | Provide tested configs |
| Upstream Changes | Breaking changes break users (#8) | Pin versions, wait before adopting |

---

## Sources

### Official Documentation
- [Docker Post-Installation](https://docs.docker.com/engine/install/linux-postinstall/)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [systemd.service Manual](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)
- [Apple launchd Documentation](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)

### Community Resources
- [Docker Permission Issues](https://eastondev.com/blog/en/posts/dev/20251217-docker-mount-permissions-guide/) (HIGH confidence)
- [systemd User Services](https://wiki.archlinux.org/title/Systemd/User) (HIGH confidence)
- [launchd Tutorial](https://www.launchd.info/) (HIGH confidence)
- [CLI UX Patterns](https://lucasfcosta.com/2022/06/01/ux-patterns-cli-tools.html) (MEDIUM confidence)
- [OWASP Web Security 2025](https://www.ateamsoftsolutions.com/web-application-security-checklist-2025-complete-owasp-top-10-implementation-guide-for-ctos/) (MEDIUM confidence)

### Research
- [Pinning Is Futile (arXiv 2025)](https://arxiv.org/html/2502.06662v1) (HIGH confidence - peer reviewed)
