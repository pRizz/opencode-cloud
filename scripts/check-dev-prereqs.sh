#!/usr/bin/env bash
set -euo pipefail

required_failures=0
warning_count=0

print_ok() {
  echo "OK: $1"
}

print_error() {
  echo "Error: $1" >&2
}

print_warn() {
  echo "Warning: $1"
}

version_parts_or_empty() {
  # Extract the leading X.Y.Z portion from versions like 1.89.0, 1.3.9-dev, etc.
  local raw="$1"
  local normalized
  normalized="$(echo "$raw" | sed -E 's/^([0-9]+)\.([0-9]+)\.([0-9]+).*$/\1.\2.\3/')" || true
  if [[ "$normalized" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "$normalized"
  fi
}

version_ge() {
  local current="$1"
  local minimum="$2"
  local c1 c2 c3 m1 m2 m3

  IFS='.' read -r c1 c2 c3 <<<"$current"
  IFS='.' read -r m1 m2 m3 <<<"$minimum"

  if (( c1 > m1 )); then return 0; fi
  if (( c1 < m1 )); then return 1; fi
  if (( c2 > m2 )); then return 0; fi
  if (( c2 < m2 )); then return 1; fi
  if (( c3 >= m3 )); then return 0; fi
  return 1
}

require_command() {
  local cmd="$1"
  local hint="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    print_ok "$cmd found"
  else
    print_error "$cmd is required for development setup."
    print_error "Install hint: $hint"
    required_failures=$((required_failures + 1))
  fi
}

warn_if_missing() {
  local cmd="$1"
  local usage="$2"
  local hint="$3"
  if command -v "$cmd" >/dev/null 2>&1; then
    print_ok "$cmd found (optional)"
  else
    print_warn "$cmd is missing (optional). Used by: $usage"
    print_warn "Install hint: $hint"
    warning_count=$((warning_count + 1))
  fi
}

echo "Checking development prerequisites..."

require_command "git" "Install Git (https://git-scm.com/downloads)."
require_command "just" "Install just: 'brew install just' or 'cargo install just' (https://github.com/casey/just)."
require_command "bun" "Install Bun (https://bun.sh)."
require_command "cargo" "Install Rust via rustup (https://rustup.rs)."
require_command "rustc" "Install Rust via rustup (https://rustup.rs)."
require_command "node" "Install Node.js 20+ (https://nodejs.org)."

if command -v node >/dev/null 2>&1; then
  node_raw="$(node --version | sed 's/^v//')"
  node_version="$(version_parts_or_empty "$node_raw")"
  if [[ -z "$node_version" ]]; then
    print_error "Could not parse Node.js version from: $node_raw"
    required_failures=$((required_failures + 1))
  elif version_ge "$node_version" "20.0.0"; then
    print_ok "node version $node_version >= 20.0.0"
  else
    print_error "node version $node_version is too old. Required: >= 20.0.0"
    required_failures=$((required_failures + 1))
  fi
fi

if command -v bun >/dev/null 2>&1; then
  bun_raw="$(bun --version | head -n1)"
  bun_version="$(version_parts_or_empty "$bun_raw")"
  if [[ -z "$bun_version" ]]; then
    print_error "Could not parse Bun version from: $bun_raw"
    required_failures=$((required_failures + 1))
  elif version_ge "$bun_version" "1.3.9"; then
    print_ok "bun version $bun_version >= 1.3.9"
  else
    print_error "bun version $bun_version is too old. Required: >= 1.3.9"
    required_failures=$((required_failures + 1))
  fi
fi

if command -v rustc >/dev/null 2>&1; then
  rustc_raw="$(rustc --version | awk '{print $2}')"
  rustc_version="$(version_parts_or_empty "$rustc_raw")"
  if [[ -z "$rustc_version" ]]; then
    print_error "Could not parse rustc version from: $rustc_raw"
    required_failures=$((required_failures + 1))
  elif version_ge "$rustc_version" "1.89.0"; then
    print_ok "rustc version $rustc_version >= 1.89.0"
  else
    print_error "rustc version $rustc_version is too old. Required: >= 1.89.0"
    required_failures=$((required_failures + 1))
  fi
fi

warn_if_missing \
  "docker" \
  "'just dev', 'just run start', 'just check-docker', 'just build-docker', and Docker-risk paths in 'just pre-commit'" \
  "Install Docker Desktop (macOS) or Docker Engine (Linux)."
warn_if_missing \
  "jq" \
  "'just lint' (via check-fork-typecheck-wiring) and 'just check-updates'" \
  "Install jq (https://jqlang.org/download/)."
warn_if_missing \
  "shellcheck" \
  "'just lint' (lint-shell)" \
  "Install shellcheck (brew/apt package managers)."
warn_if_missing \
  "actionlint" \
  "'just lint' (lint-workflows)" \
  "Install actionlint (https://github.com/rhysd/actionlint)."
warn_if_missing \
  "cfn-lint" \
  "git pre-commit hook when CloudFormation templates are staged" \
  "Install cfn-lint (https://github.com/aws-cloudformation/cfn-lint)."

echo
echo "Summary: required failures=$required_failures, optional warnings=$warning_count"

if (( required_failures > 0 )); then
  print_error "Missing required prerequisites. Fix the errors above and rerun 'just setup'."
  exit 1
fi

echo "Prerequisite check passed."
