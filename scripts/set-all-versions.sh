#!/usr/bin/env bash
set -euo pipefail

new_version="${1:-}"

if [[ -z "${new_version}" ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

if [[ ! "${new_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Invalid version: ${new_version}. Expected format: X.Y.Z" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

perl -0pi -e "s/version = \"[0-9]+\\.[0-9]+\\.[0-9]+\"/version = \"${new_version}\"/" \
  "${repo_root}/Cargo.toml"

perl -0pi -e "s/opencode-cloud-core = \\{ version = \"[0-9]+\\.[0-9]+\\.[0-9]+\"/opencode-cloud-core = { version = \"${new_version}\"/" \
  "${repo_root}/Cargo.toml"

# Update core package Cargo.toml (standalone, not workspace-inherited)
perl -pi -e "s/^version = \"[0-9]+\\.[0-9]+\\.[0-9]+\"/version = \"${new_version}\"/" \
  "${repo_root}/packages/core/Cargo.toml"

perl -0pi -e "s/\"version\": \"[0-9]+\\.[0-9]+\\.[0-9]+\"/\"version\": \"${new_version}\"/" \
  "${repo_root}/packages/cli-node/package.json"

perl -0pi -e "s/\"version\": \"[0-9]+\\.[0-9]+\\.[0-9]+\"/\"version\": \"${new_version}\"/" \
  "${repo_root}/packages/core/package.json"

echo "Updated versions to ${new_version}"
