# Project Research Summary

**Project:** opencode-cloud-service
**Domain:** Cross-platform CLI installer for containerized AI coding agent service
**Researched:** 2026-01-18
**Confidence:** MEDIUM-HIGH

## Executive Summary

This project is a cross-platform CLI installer that deploys opencode (an AI coding agent) as a persistent cloud service. The recommended approach is a monorepo structure with dual distribution paths: npm/npx for TypeScript users and cargo for Rust users. Both CLIs share identical command interfaces and delegate to platform-specific service managers (systemd on Linux, launchd on macOS, Windows SCM). Docker is the container runtime, managed programmatically via Dockerode (Node) and Bollard (Rust) rather than CLI shelling.

The architecture follows the "thin CLI, thick core" pattern used by Cloudflare's cloudflared: CLI entry points handle argument parsing and user interaction, while shared core logic manages Docker operations, service installation, and configuration persistence. Configuration uses XDG-compliant paths (env-paths for Node, directories crate for Rust) to avoid platform-specific bugs. The interactive setup wizard is a differentiator but must be paired with full non-interactive flag support for CI/CD automation.

The primary risks are: (1) Docker permission mismatches between host and container causing "Permission Denied" cascades, (2) npm global install permission hell driving users to dangerous `sudo` workarounds, (3) service installation permission errors on systemd/launchd requiring careful platform abstraction, and (4) web UI exposure without authentication creating critical security vulnerabilities. Mitigation requires defaulting to localhost binding, requiring auth before network exposure, using named volumes instead of bind mounts, and prioritizing `npx` usage over global install.

## Key Findings

### Recommended Stack

The stack prioritizes battle-tested libraries with high download counts and active maintenance. Node.js 20 LTS is required (Commander 14 dependency). TypeScript and Rust are the primary languages, each with ecosystem-standard tooling.

**Core technologies (Node.js):**
- **Commander.js 14.x**: CLI framework - 14M+ weekly downloads, excellent subcommand support
- **@inquirer/prompts**: Interactive wizard - modern rewrite of Inquirer, smaller bundle
- **dockerode 4.x**: Docker API client - most popular, promise-based, handles streams well
- **conf**: Config persistence - atomic writes, XDG-compliant paths, built for CLIs
- **ora + chalk**: Terminal UX - industry standard spinners and colors

**Core technologies (Rust):**
- **clap 4.5.x**: CLI framework - undisputed standard, derive macro for clean code
- **tokio 1.43+ LTS**: Async runtime - required for bollard, LTS until March 2026
- **bollard 0.19.x**: Docker API client - only mature async Docker client for Rust
- **dialoguer + indicatif**: Interactive prompts and progress - console-rs ecosystem
- **directories**: Config paths - cross-platform XDG resolution

**Service installation:** Prefer template-based generation with native tool spawning (`systemctl`, `launchctl`, `sc.exe`) over library abstractions for reliability and debuggability.

### Expected Features

**Must have (table stakes):**
- One-command installation (`npm install -g` / `cargo install` / `npx`) — compiles from source; Rust 1.85+ required
- Start/stop/restart/status commands
- Service logs access (`logs`, `logs -f`)
- Persistence across reboots (OS service integration)
- Port configuration with conflict detection
- Basic password authentication
- Environment variable configuration
- Uninstall/cleanup command
- Clear error messages with fix instructions
- Comprehensive `--help` documentation

**Should have (competitive):**
- Interactive setup wizard with non-interactive fallback
- Health check endpoint (`/health` or `/api/health`)
- Auto-restart on crash (systemd/launchd restart policies)
- Update command for in-place upgrades
- Pre-flight checks before installation
- Structured JSON output (`--json` flag)
- systemd/launchd unit file generation command

**Defer (v2+):**
- Automatic TLS/HTTPS (high complexity, requires domain setup)
- Docker deployment alternative (docker-compose.yml)
- Backup/restore configuration
- Multiple named instances support
- Web-based status dashboard
- Reverse proxy auto-configuration

### Architecture Approach

The architecture uses a monorepo with pnpm workspaces + Turborepo for TypeScript and Cargo workspaces for Rust. Five major components have clear boundaries: CLI entry points (thin wrappers), Docker Management (owns all Docker API calls), Service Installation (platform-specific OS service registration), Configuration Management (JSON persistence with XDG paths), and Docker Assets (static Dockerfile/compose templates).

**Major components:**
1. **CLI Entry Points** - Parse arguments, handle user prompts, format output; delegate to core logic
2. **Docker Management** - Container lifecycle, image pulling, log streaming via Dockerode/Bollard
3. **Service Installation** - Generate systemd units/launchd plists, register with OS service managers
4. **Config Management** - JSON files in XDG-compliant paths, schema validation
5. **Docker Assets** - Dockerfile, docker-compose.yml, platform-specific compose overrides

**Key patterns:**
- Platform Abstraction Layer (ServiceManager interface with SystemdManager, LaunchdManager, WindowsSCMManager implementations)
- Docker Socket Detection (auto-detect Unix socket vs named pipe vs DOCKER_HOST)
- Graceful SysV fallback for Linux systems without systemd

### Critical Pitfalls

1. **Docker permission mismatch** - Use named volumes instead of bind mounts; if bind mounts required, match UID/GID with `docker run -u $(id -u):$(id -g)`. Test as non-root user.

2. **npm global install permission hell** - Prioritize `npx` usage over global install; detect EACCES errors and provide nvm/fnm guidance; never suggest `sudo npm install`.

3. **Web UI exposed without auth** - Default to localhost binding; require authentication setup before allowing network exposure; show prominent warning when exposing without auth.

4. **systemd/launchd permission errors** - Use user-level services (`~/.config/systemd/user/` with linger, `~/Library/LaunchAgents/`); validate absolute paths; provide `install --diagnose` command.

5. **Config file location inconsistency** - Follow XDG spec from day one using env-paths (Node) and directories (Rust); never hardcode paths or use naive `~/.toolname` approach.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: CLI Foundation and Installation

**Rationale:** Installation is the first user touchpoint; getting it wrong causes abandonment. Config path decisions are hard to change later. Research shows npm permission issues are the #1 installation barrier.

**Delivers:** Working npm/cargo installation, basic command structure, XDG-compliant config management, prerequisite checks.

**Addresses features:**
- One-command installation
- Environment variable configuration
- Clear error messages
- Help documentation

**Avoids pitfalls:**
- Config file location inconsistency (#2)
- npm global install permission hell (#5)
- Cross-platform path handling bugs (#9)
- Missing prerequisite checks (#11)

### Phase 2: Docker Management

**Rationale:** Docker interaction is the core capability and must work before service installation. Permission issues here cascade into all subsequent phases.

**Delivers:** Container lifecycle management (create, start, stop, status, logs), image pulling with progress, Docker socket auto-detection.

**Uses:** dockerode (Node), bollard (Rust)

**Implements:** Docker Management component

**Avoids pitfalls:**
- Docker permission mismatch (#1)
- Docker Compose override conflicts (#14)
- Timeout too short for slow operations (#15)

### Phase 3: Service Installation

**Rationale:** Depends on working Docker management. This is the highest-risk area with the most platform variance. Research shows systemd/launchd permission errors are common.

**Delivers:** systemd/launchd service registration, auto-start configuration, uninstall cleanup.

**Uses:** Template-based unit file generation, native tool spawning

**Implements:** Service Installation component with Platform Abstraction Layer

**Avoids pitfalls:**
- systemd/launchd service installation permission errors (#4)
- Incomplete uninstaller leaves orphaned files (#6)

### Phase 4: Interactive Wizard and UX Polish

**Rationale:** Core functionality must exist before adding interactive layer. Research shows wizards that annoy power users hurt adoption.

**Delivers:** Interactive setup wizard, non-interactive flag equivalents, progress spinners, colored output.

**Uses:** @inquirer/prompts (Node), dialoguer + indicatif (Rust)

**Addresses features:**
- Interactive setup wizard
- Structured JSON output
- Pre-flight checks

**Avoids pitfalls:**
- Interactive CLI wizard frustrates power users (#7) - by building non-interactive first

### Phase 5: Security and Authentication

**Rationale:** Must be complete before any public release. Research identifies web UI exposure without auth as a critical security vulnerability with legal liability implications.

**Delivers:** Password authentication, localhost-only default binding, explicit network exposure opt-in, security headers.

**Addresses features:**
- Basic authentication
- Health check endpoint

**Avoids pitfalls:**
- Exposing web UI without authentication (#3)

### Phase 6: Update and Maintenance

**Rationale:** Deferred to post-MVP but planned. Research shows no upgrade path causes user friction and stale installations.

**Delivers:** Update command, config migration, version pinning, rollback support.

**Avoids pitfalls:**
- Upstream opencode breaking changes break users (#8)
- No upgrade path (#12)

### Phase Ordering Rationale

- **Installation before Docker:** Users must get the tool before using it; early permission issues drive abandonment
- **Docker before Services:** Service definitions invoke Docker; Docker must work first
- **Services before Wizard:** Core functionality before UX polish; wizard requires something to configure
- **Security before release:** Authentication is non-negotiable for network-exposed AI agent
- **Updates post-MVP:** Valuable but not blocking initial utility

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Service Installation):** Complex platform differences between systemd, launchd, and Windows SCM; needs detailed testing matrix and fallback strategies
- **Phase 5 (Security):** Authentication mechanism selection (password vs API key vs token); needs threat modeling

Phases with standard patterns (skip research-phase):
- **Phase 1 (CLI Foundation):** Commander.js and clap are well-documented with extensive examples
- **Phase 2 (Docker Management):** dockerode and bollard have comprehensive documentation and established patterns
- **Phase 4 (Interactive Wizard):** @inquirer/prompts and dialoguer have straightforward APIs

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified versions, download counts, maintenance status from npm/crates.io |
| Features | MEDIUM | Based on competitive analysis of Portainer, Coolify, CapRover; domain-specific |
| Architecture | MEDIUM-HIGH | Cloudflared pattern is proven; monorepo structure is standard |
| Pitfalls | MEDIUM | Mix of official docs and community experience; platform-specific details verified |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **Windows service installation:** Less tested than Linux/macOS; may need dedicated research phase
- **Specific opencode requirements:** Upstream container may have specific port/volume/capability requirements not yet researched
- **TLS integration:** Deferred to v2, but integration approach (Let's Encrypt, Caddy sidecar, manual) needs decision
- **Multi-instance support:** Deferred, but config schema should anticipate it

## Sources

### Primary (HIGH confidence)
- [Commander.js npm](https://www.npmjs.com/package/commander) - Version 14.0.2 confirmed, 14M+ weekly downloads
- [bollard crates.io](https://crates.io/crates/bollard) - Version 0.19.3, Docker API 1.49
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/) - Authoritative for config paths
- [systemd.service Manual](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html) - Official service file format
- [Apple launchd Documentation](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html) - Official plist format

### Secondary (MEDIUM confidence)
- [Cloudflared Service Management](https://deepwiki.com/cloudflare/cloudflared/2.3-service-management-commands) - Cross-platform pattern reference
- [Sentry CLI npm Distribution](https://sentry.engineering/blog/publishing-binaries-on-npm) - Binary distribution pattern (we chose compile-on-install instead)
- [systemd User ArchWiki](https://wiki.archlinux.org/title/Systemd/User) - User service best practices
- [Node.js CLI Best Practices](https://github.com/lirantal/nodejs-cli-apps-best-practices) - CLI design patterns

### Tertiary (needs validation)
- Windows service installation patterns - limited research; needs Phase 3 validation
- Upstream opencode container requirements - assumed standard Docker patterns; needs confirmation

---
*Research completed: 2026-01-18*
*Ready for roadmap: yes*
