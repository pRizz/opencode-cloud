#!/usr/bin/env bash
set -euo pipefail

# Decide whether local pre-commit should run the Docker stage check.
# Exit 0 when Docker-risk files changed, otherwise exit 1.

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Not inside a git worktree; running Docker check."
  exit 0
fi

changed_paths="$(
  {
    git diff --name-only
    git diff --name-only --cached
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u
)"

if [ -z "${changed_paths}" ]; then
  echo "No file changes detected."
  exit 1
fi

is_docker_risk_path() {
  case "$1" in
    .dockerignore) return 0 ;;
    docker-compose.yml) return 0 ;;
    packages/core/src/docker/*) return 0 ;;
    .github/workflows/ci.yml) return 0 ;;
    .github/workflows/docker-publish.yml) return 0 ;;
    .github/workflows/dockerfile-updates.yml) return 0 ;;
    .github/workflows/version-bump.yml) return 0 ;;
    scripts/check-dockerfile-updates.sh) return 0 ;;
    scripts/extract-oci-description.py) return 0 ;;
    scripts/should-run-docker-check.sh) return 0 ;;
  esac
  return 1
}

matched_paths=()
while IFS= read -r path; do
  [ -n "${path}" ] || continue
  if is_docker_risk_path "${path}"; then
    matched_paths+=("${path}")
  fi
done <<< "${changed_paths}"

if [ "${#matched_paths[@]}" -eq 0 ]; then
  echo "No Docker-risk file changes detected."
  exit 1
fi

printf 'Docker-risk file changes detected:\n'
printf '  - %s\n' "${matched_paths[@]}"
exit 0
