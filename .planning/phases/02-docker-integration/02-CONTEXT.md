# Phase 2: Docker Integration - Context

**Gathered:** 2026-01-19
**Status:** In progress (Progress feedback + Volume persistence remaining)

<domain>
## Phase Boundary

CLI can build/pull custom opencode image and manage container lifecycle. This phase creates the Docker image specification, distribution strategy, and progress feedback mechanisms.

</domain>

<decisions>
## Implementation Decisions

### Dockerfile Contents

**Base & Core:**
- Base image: Ubuntu 24.04 LTS (noble)
- opencode installation: Official install script with version pinning (`curl -fsSL https://opencode.ai/install | bash`)
- User: Non-root 'opencode' user with passwordless sudo
- Working directory: `/home/opencode`
- Multi-stage builds: Yes
- OCI labels: Yes, standard labels
- Timezone: UTC
- Locale: UTF-8
- Signal handling: tini/dumb-init
- HEALTHCHECK: Yes, HTTP check
- Entrypoint: `opencode web`
- apt cache: Clean after install
- .dockerignore: Yes, exclude build artifacts

**Shell & Terminal:**
- Default shell: zsh
- Shell framework: oh-my-zsh
- Prompt: Starship
- Terminal multiplexer: tmux

**Editors:**
- vim + neovim + nano

**Languages & Runtimes:**
- Node.js + Python + Rust + Go
- Package managers: npm + pnpm (no yarn)
- Python: uv (fast package manager)
- Version manager: mise
- Global TypeScript compiler: Yes

**Developer Tools:**
- GSD opencode plugin: https://github.com/rokicool/gsd-opencode
- Standard dev tools: curl, wget, htop, etc.
- Build tools: build-essential
- JSON/YAML: jq + yq
- Git: Basic config included
- SSH keys: Mount from host
- Docker-in-Docker: Yes, supported
- lazygit + delta + difftastic
- direnv
- fzf + fd
- Modern CLI replacements: ripgrep, bat, eza
- tree
- glow (markdown renderer)
- just (task runner)
- HTTPie
- shellcheck + shfmt
- Modern system tools: procs, dust, duf
- System monitor: btop
- GitHub CLI (gh)
- Compression: Full suite (zip, unzip, 7z, xz)
- Dotfiles: Sensible defaults included

**Database Tools:**
- SQLite (sqlite3)
- PostgreSQL client (psql)
- MySQL client

**Container/K8s Tools:**
- kubectl + helm
- k9s
- dive (Docker image explorer)

**Security Tools:**
- trivy (security scanner)
- gitleaks (secret scanner)
- hadolint (Dockerfile linter)
- age + sops (encryption)
- mkcert (local TLS)

**CI/CD Tools:**
- act (run GitHub Actions locally)

**Rust Tooling:**
- cargo-nextest, cargo-audit, cargo-deny
- sccache (compilation cache)
- mold (fast linker)
- cross (cross-compilation)

**Code Quality:**
- Formatters: prettier + black
- Linters: eslint + ruff
- Biome
- Test runners: jest/vitest/pytest (common runners)

**Other:**
- protobuf compiler (protoc) + grpcurl

**Explicitly Excluded:**
- Claude Code (opencode is enough)
- Cloud CLIs (keep container focused)
- zoxide (standard cd preferred)
- grex, navi (not commonly needed)
- jless (jq is enough)

**Deferred Tools (consider later):**
- API key handling (deferred to wizard/security phase)
- Browser capabilities (configurable via CLI)
- tokei, hyperfine, watchexec
- redis-cli
- lazydocker, tunneling tools, documentation tools
- sd, xsv, hexyl
- Bun, Deno, Zig
- Terraform, stern
- cargo-expand, cargo-bloat, cargo-machete, bacon
- IPython, mypy, poetry, venv handling
- usql, pgcli/mycli, jwt-cli
- File transfer tools, pv
- ImageMagick, FFmpeg
- typos, asciinema
- pre-commit
- mermaid-cli, pandoc
- nodemon/cargo-watch
- tldr (Claude decides)

### Image Distribution

**Registry & Naming:**
- Host on: Both GHCR and Docker Hub
- Image name: `prizz/opencode-cloud-sandbox` (Docker Hub primary), mirror `ghcr.io/prizz/opencode-cloud-sandbox`
- Registry priority: Docker Hub primary
- Fallback: Automatic failover (Docker Hub → GHCR)

**Tagging Strategy:**
- Tags: Semver + major + minor + latest + git SHA
- Pre-release tags: No, stable releases only
- Nightly builds: No, `:edge` from main is sufficient
- Tag immutability: Immutable semver, mutable rolling tags (`:latest`, `:edge`)
- Registry namespace: Single namespace, tags handle versioning

**Architecture & Build:**
- Architectures: amd64 + arm64
- Multi-platform strategy: QEMU emulation via buildx
- Build concurrency: Full matrix builds
- Build timeout: Generous (60+ minutes for QEMU)
- Platform variants: Unified image, same tools on both architectures

**Signing & Security:**
- Image signing: Yes, cosign
- Signing keys: GitHub OIDC keyless (Sigstore/Fulcio)
- Signature verification: Optional for now (verify if available, warn if not); required mode deferred
- Content trust: Both DCT + cosign
- Vulnerability scanning: Block releases on critical CVEs
- CVE rescanning: Weekly rescan of existing images
- Vulnerability allowlist: Strict, no exceptions
- SLSA provenance: Yes, generate and publish

**CI/CD:**
- Push timing: Release tags + main branch as `:edge`
- Build triggers: Tags for releases + main for `:edge`
- Release approval: Fully automated for now, consider hybrid later
- CI caching: Both GitHub Actions cache + registry cache
- Build caching: Main-based (PRs pull cache from main only)
- Cache warming: On-demand, natural activity
- Failure escalation: Auto-retry once before alerting
- Build notifications: GitHub Actions only for now
- Build secrets: No secrets in build, runtime injection only
- Build environment: Hybrid (GitHub-hosted + self-hosted for heavy tasks)
- Concurrent releases: Queue, one at a time

**Image Optimization:**
- Image size: No limit, prioritize functionality
- Squashing: No squash for now, consider post-MVP
- Startup time: Optimize for fast cold start
- Layer ordering: Balance cache efficiency and size
- Compression: gzip for now, explore zstd later
- Build context: Both minimal `.dockerignore` + selective multi-stage copies
- Content inspection: Dive in CI, fail on waste

**Cleanup & Maintenance:**
- Cleanup policy: Keep recent + milestone versions
- Base image updates: Automated weekly rebuilds
- Base image pinning: Pin to digest for reproducibility
- Dockerfile updates: Hybrid (automate base image, manual for tools)

**Metadata & Documentation:**
- Build metadata in labels: Full (git SHA, branch, build time, CI URL, builder version)
- Annotations/labels: Both (OCI annotations for registry, labels for runtime)
- README sync: Auto-push to Docker Hub via CI
- Digests: Publish SHA256 in release notes
- Build badges: Yes, shields.io (status, size, version)
- Release diff: Both layer diff + tool version changelog
- Changelog format: Keep a Changelog (keepachangelog.com)

**Verification & Testing:**
- Image testing: Smoke test before push (health endpoint check)
- Post-pull verification: Yes, verify digest matches expected
- Pull retry: Moderate (3 retries with exponential backoff)
- Reproducibility: Periodic rebuild verification
- Download resume: Trust Docker + helpful CLI error messages

**Access & Credentials:**
- Access control: Public pull, CI-only push
- Credential storage: All methods with precedence (Docker config → keychain → env vars)
- CI rate limiting: Authenticated always with registry tokens
- Audit logging: Registry built-in logs

**Monitoring & Metrics:**
- Size tracking: Track in CI, alert on significant growth
- Quota monitoring: Alert on threshold
- Registry metrics: Built-in stats, explore more later
- Cost tracking: Monitor if easy/simple
- Build log retention: GitHub default (90 days)
- Log sanitization: Auto-scrub + trust CI built-in masking

**Format & Compatibility:**
- Manifest format: OCI only for now
- Runtime compatibility: Docker only, assume Podman/containerd work
- Multi-registry consistency: Best effort (push both, warn on single failure)

**Rollback & Migration:**
- Release rollback: Both options (re-tag for emergencies, prefer new versions)
- Registry backup: Dual registry mirrors each other
- Registry migration: Automated tooling ready

**Dependency Management:**
- Dependency vendoring: None, always fetch from upstream
- Update checking: CLI check first, startup warning deferred

**Entrypoint & Debugging:**
- Entrypoint debugging: Debug flag (`--debug` or `DEBUG=1`)

**Deferred Distribution Decisions:**
- Offline/airgapped support
- Mirror registry support (enterprise/China)
- SBOM generation
- Registry webhooks
- Geographic distribution
- Broader artifact signing (tarballs, checksums)
- Image inheritance (multi-tier images)

</decisions>

<specifics>
## Specific Ideas

- Use tini/dumb-init for proper signal handling in container
- HTTP health check endpoint for HEALTHCHECK instruction
- Rich error messages with Rust compiler-style formatting
- Automatic registry failover for resilience
- Dive analysis in CI to catch layer bloat early
- Keep a Changelog format for release notes
- Full build metadata in labels for traceability

</specifics>

<deferred>
## Deferred Ideas

- API key handling in Docker image (deferred to wizard/security phase)
- Browser capabilities (configurable via CLI later)
- Many development tools listed above
- Required signature verification mode
- Offline/airgapped installation
- Mirror registry configuration
- SBOM generation
- Startup update warning
- Registry webhooks
- Geographic CDN distribution
- Pre-release image tags
- Image squashing optimization

</deferred>

---

*Phase: 02-docker-integration*
*Context gathering: 2026-01-19 (in progress)*
