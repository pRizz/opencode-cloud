# Phase 15: Prebuilt Image Option - Context

**Gathered:** 2026-01-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Give users the choice between pulling a prebuilt Docker image (fast, ~2 min) or building from source (customizable, 30-60 min depending on hardware). CI/CD already publishes prebuilt images to GHCR and Docker Hub (Phase 14). This phase adds the user-facing choice, CLI flags, and config options.

</domain>

<decisions>
## Implementation Decisions

### First-run prompt UX
- Prompt appears in setup wizard AND on first `occ start` if skipped in wizard
- Default choice is "prebuilt" (user can press Enter to accept)
- Show detailed explanation including:
  - Time estimates: prebuilt ~2 min, build from source 30-60 min (depends on hardware, involves Rust compilation)
  - Trade-offs: prebuilt = faster, build = customizable/auditable
  - Security/trust implications of prebuilt images
  - Both registries mentioned: GHCR and Docker Hub
  - Link to publish history for transparency: https://github.com/pRizz/opencode-cloud/actions/workflows/version-bump.yml
- Show specific version being pulled (e.g., "Pull prebuilt v1.0.12...")
- On pull failure: retry 3 times, then offer to build from source instead
- Users can change choice later via config OR per-invocation flags

### Flag behavior
- Rename flags for clarity (no short aliases):
  - `--pull-sandbox-image` - Pull prebuilt image
  - `--cached-rebuild-sandbox-image` - Rebuild using Docker cache
  - `--full-rebuild-sandbox-image` - Rebuild from scratch without cache
- Remove the short `--pull` and `--build` flags mentioned in roadmap (start fresh with verbose names)
- Flags are mutually exclusive - error if combined
- Default mode is pull (when image_source=prebuilt)
- Add `--no-update-check` flag to skip version checking on start
- When container is running and user uses any of these flags: prompt "Container is running. Stop and rebuild? [y/N]"
- `occ update` respects image_source config: pulls if prebuilt, rebuilds if build. Show clear message about what's happening before it starts.

### Config persistence
- Two separate config options:
  - `image_source`: `prebuilt` | `build` (where image comes from)
  - `update_check`: `always` | `once` | `never` (how often to check for updates)
- Defaults for new installs:
  - `image_source`: `prebuilt`
  - `update_check`: `always`
- Config sets default behavior, flags override per-invocation

### Version mismatch handling
- On version mismatch (CLI differs from image): prompt to update (current Phase 14 behavior)
- Version comparison: exact match required (CLI 1.0.12 requires image 1.0.12)
- If user declines update: warn once per terminal session, then silent. Include instruction for how to update later.
- For dev builds (no prebuilt available): default to latest prebuilt image version
- Always warn when using mismatched versions (e.g., "Using prebuilt v1.0.12 (CLI is v1.0.13-dev)")
- `occ status` shows image source: "Image: v1.0.12 (prebuilt from docker.io)" or "Image: v1.0.12 (built from source)"
- Store and display which registry the image was pulled from

### Claude's Discretion
- Exact prompt wording and formatting
- Retry backoff timing for failed pulls
- How to detect/store image source (labels, local state file, etc.)
- Implementation of session-based warning suppression

</decisions>

<specifics>
## Specific Ideas

- Link to GitHub Actions publish history for transparency: https://github.com/pRizz/opencode-cloud/actions/workflows/version-bump.yml
- Verbose flag names (`--pull-sandbox-image` etc.) to make it crystal clear what's being affected
- Build time is 30-60 minutes, not 10-15 as originally estimated (involves Rust compilation)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 15-prebuilt-image-option*
*Context gathered: 2026-01-24*
