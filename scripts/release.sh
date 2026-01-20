#!/usr/bin/env bash
set -euo pipefail

# Release script for opencode-cloud
#
# This script:
# 1. Updates all version numbers across the monorepo
# 2. Builds the workspace to verify compilation
# 3. Runs publish dry-run to verify packages are ready
# 4. Commits all changes (including Cargo.lock)
# 5. Creates a git tag
# 6. Pushes the tag to the remote
#
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 1.2.3

new_version="${1:-}"

if [[ -z "${new_version}" ]]; then
    echo "Usage: $0 <version>" >&2
    echo "Example: $0 1.2.3" >&2
    exit 1
fi

# Validate version format (X.Y.Z)
if [[ ! "${new_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Invalid version '${new_version}'. Expected format: X.Y.Z" >&2
    exit 1
fi

tag_name="release/v${new_version}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

# Check if tag already exists
if git rev-parse "${tag_name}" >/dev/null 2>&1; then
    echo "Error: Tag '${tag_name}' already exists" >&2
    exit 1
fi

# Check for uncommitted changes
if [[ -n "$(git status --porcelain)" ]]; then
    echo "Error: Working directory has uncommitted changes" >&2
    echo "Please commit or stash changes before releasing" >&2
    exit 1
fi

echo "==> Releasing version ${new_version}"
echo ""

# Step 1: Update versions
echo "==> Updating versions..."
./scripts/set-all-versions.sh "${new_version}"
echo ""

# Step 2: Build to verify compilation and update Cargo.lock
echo "==> Building workspace..."
just build
echo ""

# Step 3: Run publish dry-run
echo "==> Running publish dry-run..."
just publish-all-dry-run
echo ""

# Step 4: Commit all changes
echo "==> Committing changes..."
git add -A
git commit -m "chore(release): v${new_version}"
echo ""

# Step 5: Create tag
echo "==> Creating tag ${tag_name}..."
git tag "${tag_name}"
echo ""

# Step 6: Push tag
echo "==> Pushing tag to remote..."
git push origin "${tag_name}"
echo ""

echo "=========================================="
echo "Release v${new_version} complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Push the commit: git push"
echo "  2. Publish to registries: just publish-all"
echo ""
