#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dockerfile="${root_dir}/packages/core/src/docker/Dockerfile"
submodule_dir="${root_dir}/packages/opencode"
gitmodules="${root_dir}/.gitmodules"

set_output() {
  local key="$1"
  local value="$2"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "${key}=${value}" >> "${GITHUB_OUTPUT}"
  fi
}

normalize_repo_url() {
  local input="$1"
  local normalized="$input"

  if [[ "${normalized}" == git@github.com:* ]]; then
    normalized="https://github.com/${normalized#git@github.com:}"
  elif [[ "${normalized}" == ssh://git@github.com/* ]]; then
    normalized="https://github.com/${normalized#ssh://git@github.com/}"
  elif [[ "${normalized}" == git://github.com/* ]]; then
    normalized="https://github.com/${normalized#git://github.com/}"
  elif [[ "${normalized}" == http://github.com/* ]]; then
    normalized="https://github.com/${normalized#http://github.com/}"
  fi

  normalized="${normalized%.git}"
  normalized="${normalized%/}"

  echo "${normalized}"
}

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
set_output "submodule_branch" "${submodule_branch}"

submodule_repo_url_raw="$(git -C "${submodule_dir}" remote get-url origin 2>/dev/null || true)"
if [[ -z "${submodule_repo_url_raw}" && -f "${gitmodules}" ]]; then
  submodule_repo_url_raw="$(git config -f "${gitmodules}" --get submodule.packages/opencode.url 2>/dev/null || true)"
fi
submodule_repo_url="$(normalize_repo_url "${submodule_repo_url_raw}")"
set_output "submodule_repo_url" "${submodule_repo_url}"
set_output "current_submodule_commit" "${current_submodule_commit}"

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
set_output "latest_commit" "${latest_commit}"

current_pin="$(grep -oE 'OPENCODE_COMMIT="[^\"]+"' "${dockerfile}" | head -n1 || true)"
if [[ -z "${current_pin}" ]]; then
  echo "Failed to find OPENCODE_COMMIT in ${dockerfile}." >&2
  exit 1
fi

current_pin_value="${current_pin#OPENCODE_COMMIT=\"}"
current_pin_value="${current_pin_value%\"}"
set_output "current_pin_value" "${current_pin_value}"

needs_update="false"
if [[ "${latest_commit}" != "${current_submodule_commit}" ]]; then
  needs_update="true"
fi
if [[ "${latest_commit}" != "${current_pin_value}" ]]; then
  needs_update="true"
fi
set_output "needs_update" "${needs_update}"

if [[ "${needs_update}" == "false" ]]; then
  set_output "updated" "false"
  set_output "updated_pin_value" "${current_pin_value}"
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
set_output "updated" "true"
set_output "updated_pin_value" "${latest_commit}"

echo "Updated opencode references."
echo "  Branch: ${submodule_branch}"
echo "  Submodule: ${current_submodule_commit} -> ${latest_commit}"
echo "  Dockerfile pin: ${current_pin_value} -> ${latest_commit}"
echo "  Submodule: ${submodule_dir}"
echo "  Dockerfile: ${dockerfile}"
