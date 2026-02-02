#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# opencode-cloud setup script layout + fetch strategy
# =============================================================================
# Shared core: scripts/provisioning/opencode-cloud-setup.sh (this file)
# - Common provisioning: env loading, logging, idempotency, core install steps
# - No AWS-only dependencies (AWS data injected via env or wrapper)
#
# AWS wrappers:
# - scripts/provisioning/opencode-cloud-setup-cloudformation.sh
#   - CloudFormation signal + Secrets Manager write + AWS-specific validation
# - scripts/provisioning/opencode-cloud-setup-cloud-init.sh
#   - cloud-init status/motd handling (no AWS APIs)
#
# Fetch strategy (templates):
# - CloudFormation + cloud-init user-data install a tiny bootstrap at
#   /usr/local/bin/opencode-cloud-setup.sh that downloads these repo scripts
#   from a pinned Git ref (commit SHA or release tag), optionally verifies
#   sha256 checksums, then executes the appropriate wrapper.
# =============================================================================

OPENCODE_SETUP_STATUS_DIR="/var/lib/opencode-cloud"
OPENCODE_SETUP_PROVISIONED_FILE="${OPENCODE_SETUP_STATUS_DIR}/.provisioned"
OPENCODE_SETUP_LOG_FILE="/var/log/opencode-cloud-setup.log"
OPENCODE_SETUP_STACK_ENV="/etc/opencode-cloud/stack.env"
OPENCODE_SETUP_USER="${OPENCODE_SETUP_USER:-}"
OPENCODE_SETUP_HOME="${OPENCODE_SETUP_HOME:-}"

if [ -n "$OPENCODE_SETUP_USER" ] && [ -z "$OPENCODE_SETUP_HOME" ]; then
  OPENCODE_SETUP_HOME="/home/${OPENCODE_SETUP_USER}"
fi

opencode_setup_log() {
  printf '%s %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" "$*" >> "$OPENCODE_SETUP_LOG_FILE"
}

opencode_setup_log_env() {
  local home_value
  local path_value

  home_value="$(printenv HOME 2>/dev/null || true)"
  path_value="$(printenv PATH 2>/dev/null || true)"
  if [ -z "$home_value" ]; then
    opencode_setup_log "opencode-cloud setup: HOME is unset"
  else
    opencode_setup_log "opencode-cloud setup: HOME before set: $home_value"
  fi
  opencode_setup_log "opencode-cloud setup: PATH before set: $path_value"
}

opencode_setup_set_home() {
  opencode_setup_log_env
  export HOME="/root"
}

opencode_setup_prepare_status_dir() {
  opencode_setup_log "opencode-cloud setup: prepare status dir"
  mkdir -p "$OPENCODE_SETUP_STATUS_DIR"
  chmod 700 "$OPENCODE_SETUP_STATUS_DIR"
  opencode_setup_log "opencode-cloud setup: status dir ready"
}

opencode_setup_load_stack_env() {
  if [ -f "$OPENCODE_SETUP_STACK_ENV" ]; then
    opencode_setup_log "opencode-cloud setup: load stack env"
    # shellcheck disable=SC1091
    source "$OPENCODE_SETUP_STACK_ENV"
    opencode_setup_log "opencode-cloud setup: stack env loaded"
  fi
}

opencode_setup_apply_defaults() {
  : "${OPENCODE_IMAGE:=ghcr.io/prizz/opencode-cloud-sandbox:latest}"
  : "${OPENCODE_CONTAINER_NAME:=opencode-cloud-sandbox}"
  : "${OPENCODE_USERNAME:=opencode}"
  : "${OPENCODE_DOMAIN_URL:=}"
  : "${OPENCODE_ALB_URL:=}"
}

opencode_setup_is_provisioned() {
  [ -f "$OPENCODE_SETUP_PROVISIONED_FILE" ]
}

opencode_setup_mark_provisioned() {
  opencode_setup_log "opencode-cloud setup: mark provisioned"
  date -u +"%Y-%m-%dT%H:%M:%SZ" > "$OPENCODE_SETUP_PROVISIONED_FILE"
  opencode_setup_log "opencode-cloud setup: complete"
}

opencode_setup_configure_rustup_profile() {
  opencode_setup_log "opencode-cloud setup: configure rustup PATH for ubuntu"
  cat <<'EOF' > /etc/profile.d/opencode-cloud.sh
export CARGO_HOME="$HOME/.cargo"
export PATH="$PATH:$CARGO_HOME/bin"
if [ -f "$CARGO_HOME/env" ]; then
  . "$CARGO_HOME/env"
fi
EOF
  chmod 0644 /etc/profile.d/opencode-cloud.sh
  if [ -n "$OPENCODE_SETUP_HOME" ] \
      && [ -f "$OPENCODE_SETUP_HOME/.bashrc" ] \
      && ! grep -q 'opencode-cloud.sh' "$OPENCODE_SETUP_HOME/.bashrc" 2>/dev/null; then
    echo 'source /etc/profile.d/opencode-cloud.sh' >> "$OPENCODE_SETUP_HOME/.bashrc"
  fi
  opencode_setup_log "opencode-cloud setup: rustup PATH configured"
}

opencode_setup_prepare_root_rustup_path() {
  opencode_setup_log "opencode-cloud setup: set PATH"
  export PATH="$HOME/.cargo/bin:$PATH"
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
  fi
  opencode_setup_log "opencode-cloud setup: PATH set"
}

opencode_setup_run_as_user() {
  local cmd="$*"
  if [ -n "$OPENCODE_SETUP_USER" ]; then
    runuser -u "$OPENCODE_SETUP_USER" -- bash -lc \
      "export HOME=\"$OPENCODE_SETUP_HOME\"; \
      if [ -f \"$OPENCODE_SETUP_HOME/.cargo/env\" ]; then \
        . \"$OPENCODE_SETUP_HOME/.cargo/env\"; \
      fi; \
      $cmd"
  else
    bash -lc "$cmd"
  fi
}

opencode_setup_ensure_rust_toolchain() {
  if ! opencode_setup_run_as_user "command -v cargo >/dev/null 2>&1"; then
    opencode_setup_log "opencode-cloud setup: install rust toolchain"
    opencode_setup_run_as_user \
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
        bash -s -- -y --profile minimal --default-toolchain 1.88.0"
    opencode_setup_log "opencode-cloud setup: rust toolchain installed"
  fi

  if ! opencode_setup_run_as_user "command -v cargo >/dev/null 2>&1"; then
    opencode_setup_log "opencode-cloud setup: cargo still missing after install"
    return 1
  fi

  opencode_setup_log "opencode-cloud setup: current cargo path: $(opencode_setup_run_as_user 'command -v cargo || true')"
  opencode_setup_log "opencode-cloud setup: current cargo version: $(opencode_setup_run_as_user 'cargo --version || true')"
}

opencode_setup_ensure_cli() {
  if ! opencode_setup_run_as_user "command -v opencode-cloud >/dev/null 2>&1"; then
    opencode_setup_log "opencode-cloud setup: install opencode-cloud CLI"
    opencode_setup_run_as_user "cargo install opencode-cloud"
    opencode_setup_log "opencode-cloud setup: opencode-cloud CLI installed"
  fi

  OPENCODE_CLI_VERSION="$(opencode_setup_run_as_user "opencode-cloud --version 2>/dev/null || true")"
  opencode_setup_log "opencode-cloud setup: current opencode-cloud path: $(opencode_setup_run_as_user 'command -v opencode-cloud || true')"
  opencode_setup_log "opencode-cloud setup: current opencode-cloud version: ${OPENCODE_CLI_VERSION}"

  if ! opencode_setup_run_as_user "command -v opencode-cloud >/dev/null 2>&1"; then
    opencode_setup_log "opencode-cloud setup: opencode-cloud still missing after install"
    return 1
  fi
}

opencode_setup_enable_docker() {
  opencode_setup_log "opencode-cloud setup: enable docker"
  systemctl enable --now docker
  opencode_setup_log "opencode-cloud setup: docker enabled"

  if ! command -v docker >/dev/null 2>&1; then
    opencode_setup_log "opencode-cloud setup: docker CLI missing"
    return 1
  fi

  if [ -n "$OPENCODE_SETUP_USER" ] && getent group docker >/dev/null 2>&1; then
    opencode_setup_log "opencode-cloud setup: add $OPENCODE_SETUP_USER to docker group"
    usermod -aG docker "$OPENCODE_SETUP_USER"
  fi
}

opencode_setup_wait_for_docker() {
  opencode_setup_log "opencode-cloud setup: wait for docker readiness"
  for _ in $(seq 1 30); do
    if docker info >/dev/null 2>&1; then
      break
    fi
    sleep 2
  done

  if ! docker info >/dev/null 2>&1; then
    opencode_setup_log "opencode-cloud setup: docker did not become ready"
    return 1
  fi
}

opencode_setup_bootstrap_config() {
  opencode_setup_log "opencode-cloud setup: bootstrap config"
  opencode_setup_run_as_user "opencode-cloud --quiet setup --bootstrap"
  opencode_setup_log "opencode-cloud setup: bootstrap complete"
}

opencode_setup_create_user() {
  if [ "${OPENCODE_USERNAME}" = "opencode" ]; then
    opencode_setup_log "opencode-cloud setup: set password for user ${OPENCODE_USERNAME}"
    OPENCODE_PASSWORD="$(opencode_setup_run_as_user \
      "opencode-cloud user passwd \"${OPENCODE_USERNAME}\" --generate --print-password-only")"
  else
    opencode_setup_log "opencode-cloud setup: create user ${OPENCODE_USERNAME}"
    OPENCODE_PASSWORD="$(opencode_setup_run_as_user \
      "opencode-cloud user add \"${OPENCODE_USERNAME}\" --generate --print-password-only")"
  fi

  if [ -z "$OPENCODE_PASSWORD" ]; then
    opencode_setup_log "opencode-cloud setup: failed to read generated password"
    return 1
  fi

  pass_len="$(printf '%s' "$OPENCODE_PASSWORD" | wc -c | tr -d ' ')"
  printf 'user=%q pass_len=%s\n' "$OPENCODE_USERNAME" "$pass_len"
  opencode_setup_log "opencode-cloud setup: user created"
}

opencode_setup_disable_unauth_network() {
  opencode_setup_log "opencode-cloud setup: disable unauthenticated network"
  opencode_setup_run_as_user "opencode-cloud config set allow_unauthenticated_network false"
}

opencode_cloud_setup_run_common() {
  opencode_setup_prepare_status_dir
  opencode_setup_load_stack_env
  opencode_setup_apply_defaults

  if [ -n "$OPENCODE_SETUP_USER" ]; then
    opencode_setup_configure_rustup_profile
  else
    opencode_setup_prepare_root_rustup_path
  fi

  opencode_setup_ensure_rust_toolchain
  opencode_setup_ensure_cli
  opencode_setup_enable_docker
  opencode_setup_wait_for_docker
  opencode_setup_bootstrap_config
  opencode_setup_create_user
  opencode_setup_disable_unauth_network
}
