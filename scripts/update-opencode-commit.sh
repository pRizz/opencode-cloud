#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dockerfile="${root_dir}/packages/core/src/docker/Dockerfile"

if [[ ! -f "${dockerfile}" ]]; then
  echo "Dockerfile not found at ${dockerfile}" >&2
  exit 1
fi

latest_commit="$(
  git ls-remote https://github.com/pRizz/opencode.git HEAD | awk '{print $1}'
)"

if [[ -z "${latest_commit}" ]]; then
  echo "Failed to resolve latest commit for pRizz/opencode." >&2
  exit 1
fi

perl -0pi -e "s/OPENCODE_COMMIT=\"[^\"]+\"/OPENCODE_COMMIT=\"${latest_commit}\"/" "${dockerfile}"

echo "Updated OPENCODE_COMMIT to ${latest_commit}"
