#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fork_root="${repo_root}/packages/opencode/packages"

if [[ ! -d "${fork_root}" ]]; then
  echo "Missing fork package root: ${fork_root}" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to validate fork typecheck wiring." >&2
  exit 1
fi

missing=0

for pkg in "${fork_root}"/fork-*; do
  [[ -d "${pkg}" ]] || continue
  manifest="${pkg}/package.json"

  if [[ ! -f "${manifest}" ]]; then
    echo "Missing package.json for ${pkg}" >&2
    missing=1
    continue
  fi

  name="$(jq -r '.name // empty' "${manifest}")"
  typecheck_script="$(jq -r '.scripts.typecheck // empty' "${manifest}")"

  if [[ -z "${typecheck_script}" ]]; then
    echo "Missing scripts.typecheck in ${manifest}${name:+ (${name})}" >&2
    missing=1
  fi
done

if [[ "${missing}" -ne 0 ]]; then
  exit 1
fi

echo "All fork-* packages define scripts.typecheck."
