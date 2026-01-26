# Phase 18: CLI Sync Strategy - Context

**Gathered:** 2026-01-25
**Updated:** 2026-01-25 (added prebuilt binary distribution)
**Status:** Ready for additional planning

<domain>
## Phase Boundary

Establish patterns and processes to keep the Rust CLI (`packages/cli-rust`) and Node CLI (`packages/cli-node`) feature-complete and behavior-consistent. This phase creates the sync strategy AND implements zero-friction npm installation via prebuilt binaries.

**Expanded scope:** Originally prebuilt binaries were Phase 22 work, but user requested bringing it forward. The goal is `npm install` "just works" without users needing Rust installed.

</domain>

<decisions>
## Implementation Decisions

### Source of Truth
- Rust CLI is the authoritative implementation
- Node CLI delegates ALL commands to Rust via generic passthrough
- Passthrough is fully automatic - Node passes all args to Rust binary, zero configuration
- No Node-specific commands - everything delegates, including --version and --help

### Distribution Model (UPDATED)
- Node CLI bundles prebuilt Rust binaries for each platform via optionalDependencies
- Uses the esbuild/swc pattern: platform-specific packages with `os` and `cpu` fields
- `npm install` downloads only the binary for the user's platform
- No compile-on-install - users do NOT need Rust toolchain
- Fallback: Users with unsupported platforms can build from source

### Platform Support
- darwin-arm64 (Apple Silicon)
- darwin-x64 (Intel Mac)
- linux-x64-gnu (most Linux distros)
- linux-arm64-gnu (ARM Linux, including Raspberry Pi)
- linux-x64-musl (Alpine Linux)
- linux-arm64-musl (Alpine ARM)
- win32-x64 (deferred - Windows support is Phase 27)

### Package Structure
- Main package: `@opencode-cloud/cli` (or `opencode-cloud`)
- Platform packages: `@opencode-cloud/cli-darwin-arm64`, etc.
- Binary path resolution in index.js finds the installed platform package
- Uses existing pattern from packages/core/index.js as reference

### CI/CD Integration
- GitHub Actions builds binaries for all platforms
- Release workflow publishes platform packages to npm
- Each platform package contains just the binary + package.json
- Main package has optionalDependencies pointing to all platform packages

### Parity Testing
- Tests verify passthrough works (Node correctly spawns Rust and returns output)
- Cover ALL commands (not just a subset)
- Discover commands dynamically by parsing `occ --help` output
- Tests live in `packages/cli-node`
- More detailed verification (exact output, exit codes) deferred to later phase

### New Command Process
- Adding a command to Rust CLI automatically makes it available in Node CLI
- Document in: CONTRIBUTING.md + inline comments + README
- No registration required - fully automatic passthrough

### Drift Detection (CI)
- CI verifies: binary exists AND passthrough tests pass
- Failure behavior: fail the build (blocks merge/release)
- Prefer adding to existing CI workflows if feasible; otherwise create dedicated `cli-parity.yml`

### Claude's Discretion
- Exact passthrough implementation details
- Binary bundling mechanism (optionalDependencies with os/cpu fields)
- Help output parsing strategy for command discovery
- GitHub Actions workflow structure for multi-platform builds

</decisions>

<specifics>
## Specific Ideas

- "Could we make it so that the node cli essentially just delegates every argument to the rust cli?"
- "Can the delegation be made generic so that updates to the rust cli do not require code changes on the node end?"
- User wants simplicity: `npm install` should just work, no extra steps
- "we don't want the user to need to take an extra action to run the cli"
- "include a napi rust compiled binary in the npm package" - clarified to mean prebuilt CLI binary via optionalDependencies (not NAPI bindings)
- Reference: packages/core/index.js already has the optionalDependencies loader pattern

</specifics>

<deferred>
## Deferred Ideas

- Windows binaries (Phase 27)
- Compile-on-install as fallback for unsupported platforms
- More comprehensive parity testing (exact output comparison, exit code verification)
- NAPI bindings for direct Rust function calls (not needed for CLI passthrough)

</deferred>

---

*Phase: 18-cli-sync-strategy*
*Context gathered: 2026-01-25*
*Updated: 2026-01-25*
