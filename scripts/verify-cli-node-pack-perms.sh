#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

packages=(
  "cli-node-darwin-arm64"
  "cli-node-darwin-x64"
  "cli-node-linux-x64"
  "cli-node-linux-arm64"
  "cli-node-linux-x64-musl"
  "cli-node-linux-arm64-musl"
)

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

failed=0

for pkg in "${packages[@]}"; do
  pkg_dir="packages/${pkg}"
  binary_path="${pkg_dir}/bin/occ"

  echo "Checking ${pkg_dir}"

  if [[ ! -f "${binary_path}" ]]; then
    echo "  ERROR: missing binary at ${binary_path}"
    failed=1
    continue
  fi

  pack_dir="${tmp_dir}/${pkg}/pack"
  extract_dir="${tmp_dir}/${pkg}/extract"
  mkdir -p "${pack_dir}" "${extract_dir}"

  bun pm pack --cwd "${pkg_dir}" --destination "${pack_dir}" --ignore-scripts >/dev/null

  tarball="$(find "${pack_dir}" -maxdepth 1 -type f -name '*.tgz' -print -quit)"
  if [[ -z "${tarball}" ]]; then
    echo "  ERROR: no tarball produced for ${pkg_dir}"
    failed=1
    continue
  fi

  tar -xzf "${tarball}" -C "${extract_dir}"
  packed_binary="${extract_dir}/package/bin/occ"

  if [[ ! -f "${packed_binary}" ]]; then
    echo "  ERROR: packed binary missing at ${packed_binary}"
    ls -la "${extract_dir}/package/bin" || true
    echo "  tar entry:"
    tar -tvf "${tarball}" | grep 'package/bin/occ' || true
    failed=1
    continue
  fi

  if [[ -x "${packed_binary}" ]]; then
    echo "  OK: executable bit preserved"
  else
    echo "  ERROR: packed binary is not executable"
    ls -l "${packed_binary}"
    echo "  tar entry:"
    tar -tvf "${tarball}" | grep 'package/bin/occ' || true
    failed=1
  fi
done

if [[ "${failed}" -ne 0 ]]; then
  echo "CLI package tarball permission verification failed."
  exit 1
fi

echo "All CLI package tarballs have executable bin/occ."
