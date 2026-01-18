# Feature Landscape

**Domain:** Cross-platform cloud service installer for AI coding agent (opencode)
**Researched:** 2026-01-18
**Confidence:** MEDIUM (based on patterns from Portainer, Coolify, CapRover, Dokploy, and CLI service management tools)

## Table Stakes

Features users expect. Missing = product feels incomplete or users abandon.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| One-command installation | Developers expect `npm install -g` or `cargo install` to just work | Low | Standard packaging practice |
| Start/stop/restart commands | Basic service lifecycle is fundamental | Low | `opencode-cloud start`, `stop`, `restart` |
| Status command | Users need to know if service is running | Low | Show PID, uptime, port, URL |
| Service logs access | Debugging requires log visibility | Low | `opencode-cloud logs`, `logs -f` for follow |
| Persistence across reboots | Cloud services must survive restarts | Medium | Requires OS service integration (systemd/launchd) |
| Port configuration | Users need to avoid conflicts, match firewall rules | Low | Default port + override via flag/env |
| Basic authentication | Exposing AI agent without auth is a security liability | Medium | At minimum: password protection |
| Configuration via environment variables | Standard for cloud-native tooling | Low | `.env` file support + CLI overrides |
| Uninstall/cleanup command | Users need clean removal | Low | Remove service, config, optionally data |
| Cross-platform support (Linux + macOS) | Primary deployment targets | Medium | Linux is primary; macOS for dev |
| Clear error messages | Failed installations must explain why | Low | Network issues, permissions, port conflicts |
| Help documentation | `--help` must be comprehensive | Low | Commands, flags, examples |

## Differentiators

Features that set product apart. Not expected, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Interactive setup wizard** | Guides users through configuration, reduces friction | Medium | Use inquirer.js or similar; prompt for port, auth, domain |
| **Automatic TLS/HTTPS** | Security without manual certificate wrangling | High | Let's Encrypt integration; requires domain + DNS |
| **Health check endpoint** | Enables load balancer integration, monitoring | Low | `/health` or `/api/health` returning JSON status |
| **Auto-restart on crash** | Service reliability without manual intervention | Low | Built into systemd/launchd restart policies |
| **Update command** | In-place upgrades without reinstall | Medium | `opencode-cloud update` fetches new version, restarts |
| **Backup/restore configuration** | Disaster recovery, migration between servers | Medium | Export/import config file |
| **Web-based status dashboard** | Visual monitoring without CLI access | High | Adds significant complexity; may be overkill for v1 |
| **Docker deployment option** | Alternative to native install for containerized environments | Medium | Dockerfile + docker-compose.yml |
| **Reverse proxy auto-configuration** | Simplify nginx/Caddy/Traefik setup | High | Generate config snippets or integrate with proxy |
| **Resource limits configuration** | Control CPU/memory usage | Low | Expose via environment variables |
| **Multiple instances support** | Run several opencode services on one machine | Medium | Named instances with separate ports/configs |
| **Graceful shutdown** | Don't kill in-flight requests | Low | Handle SIGTERM properly |
| **API key rotation** | Security hygiene for auth credentials | Medium | Generate new keys, invalidate old |
| **Structured JSON output** | Scriptable CLI for automation | Low | `--json` flag for machine-readable output |
| **Systemd/launchd unit generation** | Users can inspect/modify service config | Low | `opencode-cloud generate-service-file` |
| **Pre-flight checks** | Verify requirements before installation | Low | Check Node.js version, port availability, permissions |

## Anti-Features

Features to explicitly NOT build. Common mistakes in this domain.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Full PaaS platform** | Scope creep; Coolify/Dokploy already exist | Focus on single-purpose: deploy opencode, nothing else |
| **Kubernetes support** | Massive complexity for a single-service installer | Docker Compose is enough; K8s users can write their own manifests |
| **Web-based configuration UI** | Adds frontend complexity, security surface, maintenance burden | CLI configuration is sufficient; optional status page only |
| **Multi-node clustering** | Beyond scope; opencode is single-instance | Recommend users run multiple separate instances if needed |
| **Database management** | Unrelated to core function | If opencode needs a DB, embed SQLite or let user bring their own |
| **Git-based deployment** | Not relevant; opencode is installed from registry, not built from source | Users install pre-built package |
| **Automatic DNS configuration** | Too many providers, too fragile | Document manual DNS setup; TLS can use DNS challenge |
| **Plugin/extension system** | Premature abstraction; adds complexity | Build specific features directly |
| **GUI installer** | Desktop apps add packaging complexity | CLI wizard is sufficient; terminal is developer comfort zone |
| **Windows Services support (v1)** | Low priority; most cloud deployments are Linux | Document WSL as workaround; add native Windows later if demanded |
| **Custom authentication providers** | OAuth, LDAP, SAML add complexity | Simple password or API key is sufficient for self-hosted |
| **Automatic firewall configuration** | Platform-specific (iptables, ufw, firewalld), can break existing rules | Document required ports; let user configure |

## Feature Dependencies

```
Installation
    |
    v
Configuration (port, auth) --> Environment Variables
    |
    v
Service Registration (systemd/launchd)
    |
    +---> Start/Stop/Status/Logs (require service to exist)
    |
    v
Persistence (reboot survival requires service registration)
    |
    v
[Optional] TLS Certificate (requires running service + domain)
    |
    v
[Optional] Health Endpoint (requires running service)
```

**Key dependencies:**
- TLS setup depends on: running service, valid domain, DNS configured
- Service management depends on: successful installation
- Logs depend on: service has run at least once
- Update depends on: ability to stop and restart service

## MVP Recommendation

For MVP (v0.1), prioritize:

### Must Have (Week 1-2)
1. **npm/cargo installation** - Table stakes; this is how users get the tool
2. **Interactive setup wizard** - Differentiator; guides port + auth configuration
3. **Start/stop/restart/status** - Table stakes; basic service lifecycle
4. **Logs command** - Table stakes; debugging capability
5. **systemd service generation** - Table stakes for persistence on Linux
6. **Basic auth (password)** - Table stakes for security
7. **Environment variable configuration** - Table stakes for cloud-native users

### Should Have (Week 3-4)
8. **launchd service generation** - Completes macOS support
9. **Health endpoint** - Enables monitoring/load balancer integration
10. **Uninstall command** - Table stakes for clean removal
11. **Pre-flight checks** - Reduces support burden from failed installs
12. **Update command** - Differentiator; reduces friction for upgrades

### Defer to Post-MVP
- **Automatic TLS**: High complexity, requires domain setup documentation
- **Docker deployment**: Alternative path, can be community-contributed
- **Backup/restore**: Nice to have after users have configs to backup
- **Multiple instances**: Edge case; most users run one instance
- **Web status dashboard**: Significant effort; CLI + health endpoint sufficient

## Competitive Analysis Summary

| Tool | Focus | Table Stakes Coverage | Key Differentiator |
|------|-------|----------------------|-------------------|
| **Portainer** | Docker management GUI | Manual container management | Multi-platform (Docker, K8s, Swarm) |
| **Coolify** | Self-hosted PaaS | Full deployment platform | Git-based auto-deploy, database provisioning |
| **CapRover** | Lightweight PaaS | One-click app deployment | Heroku-like simplicity, 512MB footprint |
| **Dokploy** | Modern PaaS | Docker + Traefik | Clean UI, real-time monitoring |
| **Easypanel** | Server control panel | Push-to-deploy, SSL, databases | Buildpacks for any language |

**Our Position:** We are NOT competing with these tools. They are full platforms. We are a single-purpose installer:

> "Install opencode as a service, configure auth, get a URL. Done."

The closest analogy is a systemd service wrapper with an interactive installer and optional HTTPS.

## Sources

**Platform Comparisons:**
- [Cloudron vs Coolify vs CapRover: 2025 PaaS Comparison](https://customworkflows.ai/blog/cloudron-vs-coolify-vs-caprover)
- [Coolify vs Dokploy: The Ultimate Comparison for Self-Hosted in 2025](https://girff.medium.com/coolify-vs-dokploy-the-ultimate-comparison-for-self-hosted-in-2025-8c63f1bda088)
- [7 Best CapRover Alternatives for Docker & Kubernetes App Hosting in 2026](https://northflank.com/blog/7-best-cap-rover-alternatives-for-docker-and-kubernetes-app-hosting-in-2025)
- [Dokploy Comparison Docs](https://docs.dokploy.com/docs/core/comparison)

**CLI Best Practices:**
- [Node.js CLI Apps Best Practices](https://github.com/lirantal/nodejs-cli-apps-best-practices)
- [Best Practices for Building CLI and Publishing to NPM](https://webbylab.com/blog/best-practices-for-building-cli-and-publishing-it-to-npm/)

**Cross-Platform Service Management:**
- [service-manager-rs (Rust)](https://github.com/chipsenkbeil/service-manager-rs) - systemd + launchd adapters
- [serviceman (Go)](https://github.com/therootcompany/serviceman) - Cross-platform CLI
- [kardianos/service (Go)](https://github.com/kardianos/service) - Windows + Linux + macOS

**Security & TLS:**
- [Let's Encrypt](https://letsencrypt.org/)
- [Smallstep step-ca](https://github.com/smallstep/certificates)
- [Best Practices for Managing Environment Variables in Self-Hosted Deployments](https://hoop.dev/blog/best-practices-for-managing-environment-variables-in-self-hosted-deployments/)

**Reverse Proxy Options:**
- [NPM, Traefik, or Caddy? How to Pick the Reverse Proxy](https://medium.com/@thomas.byern/npm-traefik-or-caddy-how-to-pick-the-reverse-proxy-youll-still-like-in-6-months-1e1101815e07)
- [Caddy Reverse Proxy in 2025](https://www.virtualizationhowto.com/2025/09/caddy-reverse-proxy-in-2025-the-simplest-docker-setup-for-your-home-lab/)

**Logging & Monitoring:**
- [Docker Service Logs Documentation](https://docs.docker.com/reference/cli/docker/service/logs/)
- [Journald Logging Driver](https://docs.docker.com/engine/logging/drivers/journald/)
- [Gatus: Self-Hosted Service Monitoring](https://www.blog.brightcoding.dev/2025/07/26/gatus-a-complete-guide-to-self-hosted-service-monitoring-and-status-pages/)
- [Healthchecks.io Self-Hosted](https://healthchecks.io/docs/self_hosted/)
