#!/usr/bin/env bash
set -euo pipefail

# opencode-cloud quick deploy
# Downloads docker-compose.yml and starts the service with persistent volumes.
# Usage: curl -fsSL https://raw.githubusercontent.com/pRizz/opencode-cloud/main/scripts/quick-deploy.sh | bash
# Interactive: curl -fsSL .../scripts/quick-deploy.sh | bash -s -- --interactive

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

COMPOSE_URL="https://raw.githubusercontent.com/pRizz/opencode-cloud/main/docker-compose.yml"
COMPOSE_FILE="docker-compose.yml"
CONTAINER_NAME="opencode-cloud-sandbox"
IOTP_MAX_WAIT_SECONDS=120
IOTP_POLL_INTERVAL=3
SERVICE_URL="http://localhost:3000"
COMPOSE_CMD=""
INTERACTIVE=false

# ---------------------------------------------------------------------------
# Terminal utilities
# ---------------------------------------------------------------------------

if [ -t 1 ] && command -v tput >/dev/null 2>&1 && [ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]; then
  COLOR_GREEN="$(tput setaf 2)"
  COLOR_YELLOW="$(tput setaf 3)"
  COLOR_RED="$(tput setaf 1)"
  COLOR_CYAN="$(tput setaf 6)"
  COLOR_BOLD="$(tput bold)"
  COLOR_RESET="$(tput sgr0)"
else
  COLOR_GREEN=""
  COLOR_YELLOW=""
  COLOR_RED=""
  COLOR_CYAN=""
  COLOR_BOLD=""
  COLOR_RESET=""
fi

info()    { printf '%s[info]%s  %s\n' "$COLOR_CYAN"   "$COLOR_RESET" "$*"; }
success() { printf '%s[ok]%s    %s\n' "$COLOR_GREEN"  "$COLOR_RESET" "$*"; }
warn()    { printf '%s[warn]%s  %s\n' "$COLOR_YELLOW" "$COLOR_RESET" "$*" >&2; }
die()     { printf '%s[error]%s %s\n' "$COLOR_RED"    "$COLOR_RESET" "$*" >&2; exit 1; }
header()  { printf '\n%s=== %s ===%s\n\n' "$COLOR_BOLD" "$*" "$COLOR_RESET"; }

# ---------------------------------------------------------------------------
# Interactive helpers
# ---------------------------------------------------------------------------

confirm() {
  if [ "$INTERACTIVE" = false ]; then
    return 0
  fi
  local prompt="$1"
  printf '%s [Y/n] ' "$prompt"
  local answer
  read -r answer </dev/tty || answer="y"
  case "$answer" in
    [nN]*) return 1 ;;
    *) return 0 ;;
  esac
}

usage() {
  cat <<'USAGE'
Usage: quick-deploy.sh [OPTIONS]

Deploy opencode-cloud using Docker Compose.

Options:
  --interactive, -i   Prompt before each major action
  --help, -h          Show this help message

Examples:
  # Fully automated (default)
  curl -fsSL https://raw.githubusercontent.com/pRizz/opencode-cloud/main/scripts/quick-deploy.sh | bash

  # Interactive mode
  curl -fsSL .../scripts/quick-deploy.sh | bash -s -- --interactive

  # Run locally
  ./scripts/quick-deploy.sh
  ./scripts/quick-deploy.sh --interactive

Docs: https://github.com/pRizz/opencode-cloud
USAGE
}

# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------

detect_os() {
  local uname_s
  uname_s="$(uname -s)"
  case "$uname_s" in
    Linux)
      if grep -qiE '(microsoft|wsl)' /proc/version 2>/dev/null; then
        echo "wsl"
      else
        echo "linux"
      fi
      ;;
    Darwin)           echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    FreeBSD)          echo "freebsd" ;;
    *)                echo "unknown" ;;
  esac
}

detect_arch() {
  local uname_m
  uname_m="$(uname -m)"
  case "$uname_m" in
    x86_64|amd64)   echo "amd64" ;;
    aarch64|arm64)   echo "arm64" ;;
    armv7l)          echo "armv7" ;;
    *)               echo "$uname_m" ;;
  esac
}

detect_linux_distro() {
  if [ -f /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    case "${ID:-}" in
      ubuntu|debian|pop|linuxmint|elementary|zorin|raspbian) echo "debian" ;;
      fedora|rhel|centos|rocky|alma|ol|amzn)                 echo "rhel" ;;
      alpine)                                                 echo "alpine" ;;
      arch|manjaro|endeavouros)                               echo "arch" ;;
      opensuse*|sles|suse)                                    echo "suse" ;;
      *)                                                      echo "${ID:-unknown}" ;;
    esac
  elif [ -f /etc/debian_version ]; then
    echo "debian"
  elif [ -f /etc/redhat-release ]; then
    echo "rhel"
  elif [ -f /etc/alpine-release ]; then
    echo "alpine"
  else
    echo "unknown"
  fi
}

# ---------------------------------------------------------------------------
# Privilege helpers
# ---------------------------------------------------------------------------

has_root() { [ "$(id -u)" -eq 0 ]; }

can_sudo() { command -v sudo >/dev/null 2>&1; }

run_privileged() {
  if has_root; then
    "$@"
  elif can_sudo; then
    sudo "$@"
  else
    die "This step requires root privileges. Re-run with sudo:
  curl -fsSL $COMPOSE_URL | sudo bash"
  fi
}

# ---------------------------------------------------------------------------
# Platform guards
# ---------------------------------------------------------------------------

check_platform() {
  local os arch
  os="$(detect_os)"
  arch="$(detect_arch)"

  case "$os" in
    windows)
      die "Windows detected. Install Docker Desktop for Windows and run this script inside WSL2.
  Download: https://www.docker.com/products/docker-desktop/"
      ;;
    macos)
      if ! command -v docker >/dev/null 2>&1; then
        die "macOS detected but Docker is not installed.
  Install Docker Desktop for Mac first:
    https://www.docker.com/products/docker-desktop/
  After installing, start Docker Desktop and re-run this script."
      fi
      if ! docker info >/dev/null 2>&1; then
        die "Docker is installed but not running. Start Docker Desktop and try again."
      fi
      success "macOS with Docker Desktop detected"
      ;;
    wsl)
      info "WSL detected"
      if ! command -v docker >/dev/null 2>&1 && ! docker info >/dev/null 2>&1; then
        die "Docker not found in WSL.
  If Docker Desktop is installed on Windows, enable WSL integration:
    Docker Desktop → Settings → Resources → WSL Integration
  Otherwise, install Docker Desktop:
    https://www.docker.com/products/docker-desktop/"
      fi
      ;;
    freebsd)
      warn "FreeBSD detected. Docker support on FreeBSD is limited — proceeding anyway."
      ;;
    linux)
      info "Linux detected"
      ;;
    *)
      warn "Unrecognized OS: $(uname -s). Proceeding anyway."
      ;;
  esac

  case "$arch" in
    amd64|arm64)
      info "Architecture: $arch"
      ;;
    armv7)
      die "ARM 32-bit (armv7) is not supported. The Docker image requires amd64 or arm64."
      ;;
    *)
      warn "Unrecognized architecture: $arch. The Docker image supports amd64 and arm64."
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Docker installation
# ---------------------------------------------------------------------------

wait_for_docker() {
  info "Waiting for Docker daemon..."
  local _attempt
  for _attempt in $(seq 1 30); do
    if docker info >/dev/null 2>&1; then
      success "Docker daemon is ready"
      return 0
    fi
    sleep 2
  done
  die "Docker daemon did not start within 60 seconds.
  Check: systemctl status docker  or  journalctl -u docker"
}

install_docker_linux() {
  local distro
  distro="$(detect_linux_distro)"
  info "Distro family: $distro"

  # Primary: official convenience script (covers most distros)
  if command -v curl >/dev/null 2>&1; then
    info "Installing Docker via get.docker.com..."
    if curl -fsSL https://get.docker.com | run_privileged sh; then
      run_privileged systemctl enable --now docker 2>/dev/null || true
      success "Docker installed via get.docker.com"
      return 0
    fi
    warn "Official Docker install script failed. Trying package manager..."
  fi

  # Fallback: distro-specific package manager
  case "$distro" in
    debian)
      run_privileged apt-get update -y
      run_privileged apt-get install -y docker.io
      run_privileged systemctl enable --now docker
      ;;
    rhel)
      run_privileged dnf install -y docker 2>/dev/null \
        || run_privileged yum install -y docker
      run_privileged systemctl enable --now docker
      ;;
    alpine)
      run_privileged apk add docker docker-cli-compose
      run_privileged rc-update add docker boot 2>/dev/null || true
      run_privileged service docker start 2>/dev/null || true
      ;;
    arch)
      run_privileged pacman -Sy --noconfirm docker docker-compose
      run_privileged systemctl enable --now docker
      ;;
    suse)
      run_privileged zypper install -y docker docker-compose
      run_privileged systemctl enable --now docker
      ;;
    *)
      die "Could not install Docker automatically for '$distro'.
  Install Docker manually: https://docs.docker.com/engine/install/
  Then re-run this script."
      ;;
  esac
  success "Docker installed via package manager"
}

ensure_docker() {
  header "Checking Docker"

  if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    success "Docker is installed and running"
    info "Docker version: $(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'unknown')"
    return 0
  fi

  if command -v docker >/dev/null 2>&1; then
    info "Docker is installed but the daemon is not running"
    if [ "$(detect_os)" = "macos" ]; then
      die "Start Docker Desktop and re-run this script."
    fi
    info "Attempting to start Docker daemon..."
    run_privileged systemctl start docker 2>/dev/null \
      || run_privileged service docker start 2>/dev/null \
      || die "Could not start Docker. Start it manually and re-run this script."
    wait_for_docker
    info "Docker version: $(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'unknown')"
    return 0
  fi

  # Docker not installed
  local os
  os="$(detect_os)"
  if [ "$os" != "linux" ] && [ "$os" != "wsl" ]; then
    die "Docker is not installed. Install Docker for your platform and re-run:
  https://docs.docker.com/engine/install/"
  fi

  if ! confirm "Docker is not installed. Install it now?"; then
    die "Docker is required. Install it manually and re-run this script."
  fi

  install_docker_linux
  wait_for_docker
  info "Docker version: $(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'unknown')"
}

# ---------------------------------------------------------------------------
# Docker Compose
# ---------------------------------------------------------------------------

ensure_compose_command() {
  if docker compose version >/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
  elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE_CMD="docker-compose"
  else
    die "Neither 'docker compose' (plugin) nor 'docker-compose' (standalone) found.
  Install Docker Compose: https://docs.docker.com/compose/install/"
  fi
  local compose_version
  compose_version="$($COMPOSE_CMD version --short 2>/dev/null || echo 'unknown')"
  success "Compose command: $COMPOSE_CMD (v$compose_version)"
}

download_compose_file() {
  header "Docker Compose File"

  if [ -f "$COMPOSE_FILE" ]; then
    info "$COMPOSE_FILE already exists — using existing file"
    return 0
  fi

  if ! confirm "Download $COMPOSE_FILE from GitHub?"; then
    die "$COMPOSE_FILE is required. Download it manually:
  curl -fsSL -o $COMPOSE_FILE $COMPOSE_URL"
  fi

  info "Downloading $COMPOSE_FILE..."
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$COMPOSE_FILE" "$COMPOSE_URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$COMPOSE_FILE" "$COMPOSE_URL"
  else
    die "Neither curl nor wget found. Install one and re-run."
  fi
  success "Downloaded $COMPOSE_FILE"
}

# ---------------------------------------------------------------------------
# Container helpers
# ---------------------------------------------------------------------------

get_container_id() {
  docker ps --filter "name=^${CONTAINER_NAME}$" --format '{{.ID}}' 2>/dev/null || true
}

# Coupling: queries opencode-cloud-bootstrap status inside the container.
# The JSON contract (ok, active, reason, otp fields) is defined in
# opencode-cloud-bootstrap.sh emit_status(). Do not change that contract
# without updating this function.
#
# --include-secret returns the raw IOTP value in the "otp" field when
# bootstrap is active. This is safe because the caller already has
# docker exec access to the container.
query_bootstrap_status() {
  docker exec "$CONTAINER_NAME" \
    /usr/local/bin/opencode-cloud-bootstrap status --include-secret 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# Service lifecycle
# ---------------------------------------------------------------------------

start_services() {
  header "Starting opencode-cloud"

  local container_id
  container_id="$(get_container_id)"

  if [ -n "$container_id" ]; then
    local image
    image="$(docker inspect --format '{{.Config.Image}}' "$container_id" 2>/dev/null || echo "unknown")"
    info "Container '$CONTAINER_NAME' is already running"
    info "  Container ID: $container_id"
    info "  Image:        $image"
    info "  Restart: $COMPOSE_CMD restart"
    info "  Stop:    $COMPOSE_CMD down"
    return 0
  fi

  if ! confirm "Start opencode-cloud?"; then
    info "Skipping service start. Run manually: $COMPOSE_CMD up -d"
    return 0
  fi

  $COMPOSE_CMD up -d
  container_id="$(get_container_id)"
  success "Services started (container: ${container_id:-unknown})"
}

# ---------------------------------------------------------------------------
# Status check and IOTP extraction
# ---------------------------------------------------------------------------

check_status_and_iotp() {
  header "Checking Setup Status"

  local elapsed=0 status_json="" active="" reason="" iotp=""

  info "Waiting for container to initialize..."
  while [ "$elapsed" -lt "$IOTP_MAX_WAIT_SECONDS" ]; do
    status_json="$(query_bootstrap_status)"
    if [ -n "$status_json" ]; then
      active="$(printf '%s' "$status_json" | jq -r '.active // empty' 2>/dev/null || true)"
      reason="$(printf '%s' "$status_json" | jq -r '.reason // empty' 2>/dev/null || true)"

      if [ "$active" = "true" ]; then
        # IOTP is active — extract value directly from JSON
        iotp="$(printf '%s' "$status_json" | jq -r '.otp // empty' 2>/dev/null || true)"
        if [ -n "$iotp" ]; then
          printf '\n' >&2
          display_fresh_setup "$iotp"
          return 0
        fi
        # otp field missing (shouldn't happen with --include-secret, but
        # keep polling in case of a race during container startup)
      fi

      case "$reason" in
        user_exists)
          printf '\n' >&2
          display_already_configured
          return 0
          ;;
        completed)
          printf '\n' >&2
          display_setup_complete
          return 0
          ;;
        not_initialized)
          ;; # container still starting up, keep polling
        invalid_state|invalid_secret)
          printf '\n' >&2
          warn "Bootstrap state is corrupted (reason: $reason)."
          warn "Reset with: docker exec $CONTAINER_NAME /usr/local/bin/opencode-cloud-bootstrap reset"
          display_ready_generic
          return 0
          ;;
        *)
          ;; # unknown reason, keep polling
      esac
    fi

    sleep "$IOTP_POLL_INTERVAL"
    elapsed=$((elapsed + IOTP_POLL_INTERVAL))
    printf '.' >&2
  done

  printf '\n' >&2
  warn "Container did not produce bootstrap status within ${IOTP_MAX_WAIT_SECONDS}s."
  warn "Check status manually:"
  warn "  docker exec $CONTAINER_NAME /usr/local/bin/opencode-cloud-bootstrap status --include-secret"
}

# ---------------------------------------------------------------------------
# Display banners
# ---------------------------------------------------------------------------

display_iotp_management() {
  local exec_prefix="docker exec $CONTAINER_NAME"
  printf '  %sIOTP management:%s\n' "$COLOR_BOLD" "$COLOR_RESET"
  printf '    Check status:  %s%s /usr/local/bin/opencode-cloud-bootstrap status%s\n' "$COLOR_CYAN" "$exec_prefix" "$COLOR_RESET"
  printf '    Reset IOTP:    %s%s /usr/local/bin/opencode-cloud-bootstrap reset%s\n' "$COLOR_CYAN" "$exec_prefix" "$COLOR_RESET"
  printf '\n'
}

display_useful_commands() {
  printf '  %sUseful commands:%s\n' "$COLOR_BOLD" "$COLOR_RESET"
  printf '    View logs:     docker logs -f %s\n' "$CONTAINER_NAME"
  printf '    Stop service:  %s down\n' "$COMPOSE_CMD"
  printf '    Restart:       %s restart\n' "$COMPOSE_CMD"
  printf '    Update image:  %s pull && %s up -d\n' "$COMPOSE_CMD" "$COMPOSE_CMD"
  printf '\n'
  printf '  Docs: https://github.com/pRizz/opencode-cloud\n'
  printf '\n'
}

display_fresh_setup() {
  local iotp="$1"
  printf '\n'
  printf '%s%s\n' "$COLOR_GREEN" "$COLOR_BOLD"
  printf '========================================================\n'
  printf '  opencode-cloud is ready!\n'
  printf '========================================================\n'
  printf '%s\n' "$COLOR_RESET"
  printf '\n'
  printf '  %sInitial One-Time Password (IOTP):%s\n' "$COLOR_BOLD" "$COLOR_RESET"
  printf '\n'
  printf '    %s%s%s%s\n' "$COLOR_CYAN" "$COLOR_BOLD" "$iotp" "$COLOR_RESET"
  printf '\n'
  printf '  %sNext steps:%s\n' "$COLOR_BOLD" "$COLOR_RESET"
  printf '    1. Open %s%s%s\n' "$COLOR_CYAN" "$SERVICE_URL" "$COLOR_RESET"
  printf '    2. Enter the IOTP above on the first-time setup panel\n'
  printf '    3. Enroll a passkey or create a username/password\n'
  printf '\n'
  printf '  The IOTP is deleted after successful setup.\n'
  printf '\n'
  display_useful_commands
}

display_already_configured() {
  printf '\n'
  printf '%s%s\n' "$COLOR_GREEN" "$COLOR_BOLD"
  printf '========================================================\n'
  printf '  opencode-cloud is ready!\n'
  printf '========================================================\n'
  printf '%s\n' "$COLOR_RESET"
  printf '\n'
  printf '  A user account is already configured.\n'
  printf '  No Initial One-Time Password is needed.\n'
  printf '\n'
  printf '  Open %s%s%s and sign in with your existing credentials.\n' "$COLOR_CYAN" "$SERVICE_URL" "$COLOR_RESET"
  printf '\n'
  display_iotp_management
  display_useful_commands
}

display_setup_complete() {
  printf '\n'
  printf '%s%s\n' "$COLOR_GREEN" "$COLOR_BOLD"
  printf '========================================================\n'
  printf '  opencode-cloud is ready!\n'
  printf '========================================================\n'
  printf '%s\n' "$COLOR_RESET"
  printf '\n'
  printf '  First-time setup was previously completed.\n'
  printf '  No Initial One-Time Password is needed.\n'
  printf '\n'
  printf '  Open %s%s%s and sign in with your credentials.\n' "$COLOR_CYAN" "$SERVICE_URL" "$COLOR_RESET"
  printf '\n'
  display_iotp_management
  display_useful_commands
}

display_ready_generic() {
  printf '\n'
  printf '  Open %s%s%s to access opencode-cloud.\n' "$COLOR_CYAN" "$SERVICE_URL" "$COLOR_RESET"
  printf '\n'
  display_useful_commands
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
  for arg in "$@"; do
    case "$arg" in
      --interactive|-i) INTERACTIVE=true ;;
      --help|-h) usage; exit 0 ;;
      *) die "Unknown option: $arg. Use --help for usage." ;;
    esac
  done

  header "opencode-cloud Quick Deploy"
  info "https://github.com/pRizz/opencode-cloud"

  check_platform
  ensure_docker
  ensure_compose_command
  download_compose_file
  start_services
  check_status_and_iotp
}

main "$@"
