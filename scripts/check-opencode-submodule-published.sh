#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: check-opencode-submodule-published.sh [--from-index] [--commit <sha>]

Checks that the opencode submodule commit pinned by the superproject is fetchable
from the configured submodule remote.

Options:
  --from-index    Read the submodule gitlink commit from the index (:packages/opencode).
  --commit <sha>  Validate an explicit commit SHA.
  -h, --help      Show this help text.
EOF
}

from_index="false"
target_commit=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-index)
      from_index="true"
      shift
      ;;
    --commit)
      if [[ $# -lt 2 ]]; then
        echo "Error: --commit requires a SHA argument." >&2
        usage >&2
        exit 1
      fi
      target_commit="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Error: Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "${target_commit}" ]]; then
  if [[ "${from_index}" == "true" ]]; then
    target_commit="$(git rev-parse --verify --quiet :packages/opencode || true)"
    if [[ -z "${target_commit}" ]]; then
      echo "Error: Could not resolve staged gitlink for packages/opencode from index." >&2
      exit 1
    fi
  else
    target_commit="$(git rev-parse --verify --quiet HEAD:packages/opencode || true)"
    if [[ -z "${target_commit}" ]]; then
      echo "Error: Could not resolve gitlink for packages/opencode from HEAD." >&2
      exit 1
    fi
  fi
fi

if [[ ! "${target_commit}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Error: Invalid commit SHA: ${target_commit}" >&2
  exit 1
fi

submodule_url="$(git config -f .gitmodules --get submodule.packages/opencode.url || true)"
if [[ -z "${submodule_url}" ]]; then
  echo "Error: Missing submodule.packages/opencode.url in .gitmodules." >&2
  exit 1
fi

probe_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${probe_dir}"
}
trap cleanup EXIT

stderr_file="${probe_dir}/fetch.stderr"
git -C "${probe_dir}" init -q

# The submodule URL in .gitmodules is SSH-style (git@github.com:...).
# CI runners and local hooks may not have an SSH key configured, so rewrite
# GitHub SSH URLs to HTTPS for this reachability probe.
if ! git \
  -C "${probe_dir}" \
  -c protocol.version=2 \
  -c url."https://github.com/".insteadOf=git@github.com: \
  -c url."https://github.com/".insteadOf=ssh://git@github.com/ \
  fetch --depth=1 "${submodule_url}" "${target_commit}" >/dev/null 2>"${stderr_file}"; then
  echo "Error: packages/opencode commit is not fetchable from remote." >&2
  echo "  Commit: ${target_commit}" >&2
  echo "  Remote: ${submodule_url}" >&2
  if [[ -s "${stderr_file}" ]]; then
    echo "  Git fetch error:" >&2
    sed 's/^/    /' "${stderr_file}" >&2
  fi
  echo "Remediation:" >&2
  echo "  1) Push the submodule commit to ${submodule_url}, or" >&2
  echo "  2) Re-pin packages/opencode to a published commit." >&2
  exit 1
fi

echo "OK: packages/opencode commit is published and fetchable."
echo "  Commit: ${target_commit}"
echo "  Remote: ${submodule_url}"
