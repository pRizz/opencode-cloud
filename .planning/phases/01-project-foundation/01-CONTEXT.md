# Phase 1: Project Foundation - Context

**Gathered:** 2026-01-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish monorepo structure with working npm and cargo CLI entry points that can read/write configuration. This phase creates the project scaffolding, build tooling, and config management foundation.

</domain>

<decisions>
## Implementation Decisions

### Project Naming
- Project name: **opencode-cloud** (renamed from opencode-cloud-service)
- npm package: `opencode-cloud`
- crates.io package: `opencode-cloud`
- CLI command: `opencode-cloud` with alias `occ`
- Update README and all documentation to reflect new name

### Monorepo Structure
- Architecture: Rust core + Node bindings
- No prebuilt binaries: Both npm and crates.io packages compile from source (Rust 1.85+ required)
- Rationale: Improved transparency and trust; users can verify exactly what they're running
- Directory structure: `packages/` layout
  - `packages/cli-node/` - Node.js CLI wrapper
  - `packages/cli-rust/` - Rust CLI implementation
  - `packages/core/` - Shared Rust core library
- Shared assets at root level: `docker/`, `schemas/`, `scripts/`
- Build tooling: pnpm workspaces + Cargo workspaces + just file for coordination
- Clear README instructions for building and installing

### Versioning
- Lockstep versioning: npm and cargo packages always same version
- Single source of truth: Claude's discretion on where version lives (likely root VERSION file or Cargo.toml)

### CI/CD
- Platform: GitHub Actions
- PR checks: lint + format, build both packages, run tests
- Release trigger: Claude's discretion (likely git tags)

### Tests
- Unit tests: colocated with source
- Integration tests: separate `tests/` directory
- Just targets: `just test`, `just test-all`

### Languages and Tooling
- Rust edition: 2024 (requires Rust 1.82+)
- TypeScript: strict mode
- ESM only for Node package
- Node version: Claude's discretion (likely Node 20 LTS+)
- MSRV: Claude's discretion (based on 2024 edition)

### Repository Configuration
- License: MIT
- Git branch strategy: main only
- Commit style: Conventional Commits (enables auto-changelog)
- Changelog: auto-generated from commits
- Gitignore: Standard Node + Rust templates combined
- Pre-commit hooks: Claude's discretion
- Editor config: Claude's discretion
- README badges: Yes (CI status, npm version, crates.io version, license)
- Issue/PR templates: Yes
- CONTRIBUTING.md: Yes
- CODE_OF_CONDUCT.md: Later
- SECURITY.md: Later
- Funding: Later
- DevContainer: Later
- Nix flake: Later
- Dependency updates: Claude's discretion

### Just File
- Targets: `build`, `build-all`, `test`, `test-all`, `lint`, `fmt`
- Additional targets: Claude's discretion

### CLI Invocation Style
- Command structure: Subcommands (`occ start`, `occ stop`, `occ config set`)
- Subcommand aliases: Yes (e.g., `st` for `status`)
- Default verbosity: Moderate (clear progress, key info)
- Colors: Auto-detect (colors when TTY, plain otherwise)
- Progress indicators: Spinners for quick ops, progress bars for long ops
- Error formatting: Rich errors with colors, suggestions, context (Rust compiler style)
- Shell completions: Claude's discretion
- Help style: clap-style formatting
- Version output: Extended (version + build info + git commit)

### CLI Global Flags
- `--verbose / -v` - Increase verbosity
- `--quiet / -q` - Suppress non-error output
- `--no-color` - Disable colors
- `--log-level=debug|info|warn|error`
- Additional global flags: Claude's discretion

### CLI Behavior
- First-run detection: No config file = launch wizard
- Confirmation prompts: Only for `uninstall` and `config reset`
- Environment variable overrides: Yes, all config values (`OCC_PORT`, etc.)
- Config precedence: Claude's discretion (likely CLI > env > file)
- Exit codes: Claude's discretion (POSIX conventions)
- Fail fast: Stop on first error
- Graceful shutdown: Cleanup on SIGINT/SIGTERM
- Timeouts: Sensible defaults per operation
- Network errors: Retry 3x with exponential backoff
- Logs: stdout/stderr only (no file)
- Docker socket: Auto-detect standard locations
- Root/sudo: Refuse by default, allow for specific operations that need it
- Docker permission errors: Rich guidance with specific fix instructions
- Encoding: UTF-8 everywhere
- Proxy: Respect HTTP_PROXY/HTTPS_PROXY
- TTY detection: Auto-adapt (no colors/progress bars when piped)
- Pager: Claude's discretion
- Terminal width: Claude's discretion
- Telemetry: None
- Banner: Yes, ASCII art on startup/wizard
- Update checks: Later (after MVP)
- JSON output (--json): Later
- Man pages: Later
- --config flag: Later
- --dry-run: Later

### Config File Conventions
- File name: `config.json` (in dedicated `~/.config/opencode-cloud/` directory)
- Location: XDG-compliant paths
  - Linux: `~/.config/opencode-cloud/config.json`
  - macOS: `~/.config/opencode-cloud/config.json`
  - Windows: `%APPDATA%\opencode-cloud\config.json`
- Format: JSONC (JSON with comments)
- Default values: port 8080, bind localhost, auto-restart enabled, plus Claude's discretion
- Sensitive data: Prefer system keychain if available, also allow plain text in config, user-configurable, always inform user how data is stored
- Validation: Strict (reject unknown keys, enforce types)
- Invalid config: Fail with clear error, refuse to start
- Version field: Yes (`{ "version": 1, ... }`)
- Migration: Auto-migrate old versions
- Backup: Create `config.json.bak` before modifying
- Env overrides: All config values overridable via `OCC_*` env vars
- Example template: Yes, ship `config.example.json` with documented options
- Schema location: `schemas/config.schema.json` in repository
- File permissions: Claude's discretion (likely 600 for security)
- Hot-reload: Later (after MVP)
- Import/export commands: Later (ties to v2 Google Drive sync)

### Data Directory
- Location: XDG data directory
  - Linux: `~/.local/share/opencode-cloud/`
  - macOS: `~/.local/share/opencode-cloud/`
  - Windows: `%LOCALAPPDATA%\opencode-cloud\`
- Inform user where data is stored

### Singleton Enforcement
- Mechanism: Lock file with PID
- Lock file location: `~/.local/share/opencode-cloud/opencode-cloud.pid`
- Stale lock handling: Check if PID is still running, remove if not
- Commands when not running: Clear error with guidance ("Service not running. Start with: occ start")

### Claude's Discretion
- Version source of truth location
- Node version minimum
- MSRV policy
- Release trigger mechanism
- Pre-commit hooks tooling
- Editor config inclusion
- Dependency update automation
- Additional just targets
- Shell completions
- Debug flag mechanism
- Exit codes
- Config precedence
- Pager usage
- Terminal width adaptation
- Config file permissions

</decisions>

<specifics>
## Specific Ideas

- "Rust core + Node bindings" architecture allows sharing logic
- "Super simple and clear README instructions" for building and installing
- Errors should feel like Rust compiler output (rich, helpful, colored)
- ASCII art banner on startup and wizard
- Make it clear to users where config and data are stored
- Keychain for secrets when available, with transparency about storage

</specifics>

<deferred>
## Deferred Ideas

- JSON output (--json flag) - Later after MVP
- Man pages - Later after MVP
- --config flag for alternate config location - Later
- --dry-run flag - Later
- Update checks on startup - Later after MVP
- Config hot-reload - Later after MVP
- Config import/export commands - v2 (ties to Google Drive sync)
- DevContainer configuration - Later
- Nix flake - Later
- CODE_OF_CONDUCT.md - Later when community grows
- SECURITY.md - Later when project is established
- GitHub Sponsors/funding - Later

</deferred>

---

*Phase: 01-project-foundation*
*Context gathered: 2026-01-18*
