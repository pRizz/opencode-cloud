# Phase 14: Versioning and Release Automation

## Context

This phase combines version tracking in the CLI with CI/CD automation for Docker images. Previously Phase 19 (CI/CD Automation) was merged into this phase.

## Goals

1. **GitHub Actions for Docker image builds** - Automated builds on release, multi-arch (amd64, arm64) via buildx, push to Docker Hub (primary) with GHCR mirror fallback
2. **Version detection in CLI** - Detect CLI/image version mismatch, prompt user to pull new image
3. **Version bump workflow** - Automated workflow to bump versions across all files, create git tags

## Current State

### Existing Infrastructure

**Version management:**
- Current version: 1.0.8 (Rust and Node packages)
- `scripts/set-all-versions.sh` - Updates version in all Cargo.toml and package.json files
- Version in 5 locations:
  - `Cargo.toml` (workspace root)
  - `Cargo.toml` (opencode-cloud-core dependency reference)
  - `packages/core/Cargo.toml`
  - `packages/cli-node/package.json`
  - `packages/core/package.json`

**Existing GitHub Actions:**
- `.github/workflows/ci.yml` - Build/test on push to main (Linux + macOS)
- `.github/workflows/publish.yml` - Publish to crates.io and npm on `release/v*` tags
- `.github/workflows/dockerfile-updates.yml` - Weekly version checks for Dockerfile tools

**Docker image:**
- Embedded in `packages/core/src/docker/Dockerfile`
- Has OCI labels but no version label
- Image name: `opencode-cloud-sandbox` (local build only in the pre-publish baseline)

### Missing Capabilities

1. No Docker image publishing automation to Docker Hub/GHCR
2. No version label in Docker image
3. CLI cannot detect image version
4. No automated version bump workflow

## Decisions

### Docker Registry
- Use Docker Hub as primary registry at `prizz/opencode-cloud-sandbox`
- Keep GHCR mirror/fallback at `ghcr.io/prizz/opencode-cloud-sandbox`
- Public access for pulling images
- Automated push on release tags

### Version Labels
- Add `org.opencode-cloud.version` label to Dockerfile
- Read from label at runtime via Docker API
- VERSION file in image as fallback (`/etc/opencode-cloud-version`)

### Mismatch Handling
- On `occ start`, compare CLI version with image version label
- If mismatch: prompt user with options (pull new image, rebuild from source, ignore)
- Add `--ignore-version` flag to bypass check
- `occ status` displays image version alongside CLI version

### Version Bump Automation
- GitHub Actions workflow with `workflow_dispatch` trigger
- User selects bump type: major, minor, patch
- Workflow:
  1. Calculate new version
  2. Run set-all-versions.sh
  3. Update CHANGELOG.md (if exists)
  4. Commit changes
  5. Create git tag (`v{version}` and `release/v{version}`)
  6. Push tag triggers existing publish.yml

## Implementation Plans

### 14-01: GitHub Actions Docker Build Workflow
- Create docker-publish.yml workflow
- Trigger on release tags (same as publish.yml)
- Build multi-arch image via buildx (amd64, arm64)
- Add version label at build time (from git tag)
- Push to Docker Hub (`prizz/opencode-cloud-sandbox`) with version tag and `:latest`, and keep GHCR mirror tags in sync

### 14-02: Version Detection in CLI
- Add version label to Dockerfile template (build-time substitution)
- Extend DockerClient to read image labels
- Add version check on `start` command
- Display image version in `status` output
- Add `--ignore-version` flag

### 14-03: Version Bump Workflow
- Create version-bump.yml workflow (workflow_dispatch)
- Inputs: bump type (major/minor/patch)
- Calculate new version using semver
- Run set-all-versions.sh
- Commit and create tags
- Trigger downstream workflows

## Success Criteria

1. GitHub Action workflow builds and pushes Docker images to Docker Hub on release (with GHCR mirror)
2. Multi-arch images (amd64, arm64) built via buildx
3. Images tagged with version (e.g., `prizz/opencode-cloud-sandbox:1.0.8`) and `:latest` (with matching GHCR mirror tags)
4. Docker images include version label (`org.opencode-cloud.version`)
5. On `occ start`, CLI detects if image version differs from CLI version
6. User is prompted to pull new image when version mismatch detected
7. `occ status` shows image version alongside CLI version
8. Workflow for version bumps with user input (major/minor/patch selection)
9. Version bump updates all relevant files and creates git tag

## Technical Notes

### Multi-arch Build Strategy

buildx can build multi-arch images using:
1. **QEMU emulation** - Slower but works on single-arch runners
2. **Native runners** - Faster but requires ARM64 runner

We'll use QEMU for simplicity since Docker builds are already slow (~15 min).

### Version Injection at Build Time

Pass version as build arg:
```dockerfile
ARG OPENCODE_CLOUD_VERSION=dev
LABEL org.opencode-cloud.version="${OPENCODE_CLOUD_VERSION}"
RUN echo "${OPENCODE_CLOUD_VERSION}" > /etc/opencode-cloud-version
```

Workflow extracts version from git tag (strips `release/v` prefix).

### Image Version Detection

Use Docker API to read labels from local image:
```rust
let inspect = docker.inspect_image("prizz/opencode-cloud-sandbox:latest").await?;
let version = inspect.config.labels.get("org.opencode-cloud.version");
```
