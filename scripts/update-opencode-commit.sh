#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dockerfile="${root_dir}/packages/core/src/docker/Dockerfile"
submodule_dir="${root_dir}/packages/opencode"
gitmodules="${root_dir}/.gitmodules"

if [[ ! -f "${dockerfile}" ]]; then
  echo "Dockerfile not found at ${dockerfile}" >&2
  exit 1
fi

if [[ ! -d "${submodule_dir}" || (! -f "${submodule_dir}/.git" && ! -d "${submodule_dir}/.git") ]]; then
  echo "Submodule is not initialized at ${submodule_dir}." >&2
  echo "Run: git submodule update --init --recursive" >&2
  exit 1
fi

dirty_state="$(git -C "${submodule_dir}" status --porcelain)"
if [[ -n "${dirty_state}" ]]; then
  echo "Submodule ${submodule_dir} has uncommitted changes; clean or stash first." >&2
  echo "${dirty_state}" >&2
  exit 1
fi

current_submodule_commit="$(git -C "${submodule_dir}" rev-parse HEAD)"

submodule_branch="dev"
if [[ -f "${gitmodules}" ]]; then
  configured_branch="$(git config -f "${gitmodules}" --get submodule.packages/opencode.branch 2>/dev/null || true)"
  if [[ -n "${configured_branch}" ]]; then
    submodule_branch="${configured_branch}"
  fi
fi

# In GitHub Actions, submodule URLs may be SSH-style from .gitmodules
# (git@github.com:...), but runners usually do not have an SSH key.
# Rewrite those URLs to HTTPS for this fetch so CI can read the repo.
git \
  -C "${submodule_dir}" \
  -c url."https://github.com/".insteadOf=git@github.com: \
  -c url."https://github.com/".insteadOf=ssh://git@github.com/ \
  fetch --prune origin "${submodule_branch}"
latest_commit="$(git -C "${submodule_dir}" rev-parse FETCH_HEAD)"
if [[ -z "${latest_commit}" ]]; then
  echo "Failed to resolve latest commit for branch ${submodule_branch}." >&2
  exit 1
fi

current_pin="$(grep -oE 'OPENCODE_COMMIT="[^\"]+"' "${dockerfile}" | head -n1 || true)"
if [[ -z "${current_pin}" ]]; then
  echo "Failed to find OPENCODE_COMMIT in ${dockerfile}." >&2
  exit 1
fi

current_pin_value="${current_pin#OPENCODE_COMMIT=\"}"
current_pin_value="${current_pin_value%\"}"

needs_update="false"
if [[ "${latest_commit}" != "${current_submodule_commit}" ]]; then
  needs_update="true"
fi
if [[ "${latest_commit}" != "${current_pin_value}" ]]; then
  needs_update="true"
fi

if [[ "${needs_update}" == "false" ]]; then
  echo "No update needed; opencode is already at the latest commit."
  echo "  Branch: ${submodule_branch}"
  echo "  Commit: ${latest_commit}"
  echo "  Submodule: ${submodule_dir}"
  echo "  Dockerfile: ${dockerfile}"
  exit 0
fi

git -C "${submodule_dir}" checkout --detach "${latest_commit}"

perl -0pi -e "s/OPENCODE_COMMIT=\"[^\"]+\"/OPENCODE_COMMIT=\"${latest_commit}\"/" "${dockerfile}"

expected_pin="OPENCODE_COMMIT=\"${latest_commit}\""
updated_pin="$(grep -oE 'OPENCODE_COMMIT="[^\"]+"' "${dockerfile}" | head -n1 || true)"
if [[ "${updated_pin}" != "${expected_pin}" ]]; then
  echo "Failed to update OPENCODE_COMMIT in ${dockerfile}." >&2
  exit 1
fi

echo "Updated opencode references."
echo "  Branch: ${submodule_branch}"
echo "  Submodule: ${current_submodule_commit} -> ${latest_commit}"
echo "  Dockerfile pin: ${current_pin_value} -> ${latest_commit}"
echo "  Submodule: ${submodule_dir}"
echo "  Dockerfile: ${dockerfile}"
