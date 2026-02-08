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
# Must match entrypoint.sh greppable_iotp_prefix
IOTP_GREP_PATTERN="INITIAL ONE-TIME PASSWORD (IOTP): "
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
  success "Compose command: $COMPOSE_CMD"
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
# Service lifecycle
# ---------------------------------------------------------------------------

start_services() {
  header "Starting opencode-cloud"

  if docker ps --format '{{.Names}}' 2>/dev/null | grep -qxF "$CONTAINER_NAME"; then
    info "Container '$CONTAINER_NAME' is already running"
    info "  Restart: $COMPOSE_CMD restart"
    info "  Stop:    $COMPOSE_CMD down"
    return 0
  fi

  if ! confirm "Start opencode-cloud?"; then
    info "Skipping service start. Run manually: $COMPOSE_CMD up -d"
    return 0
  fi

  $COMPOSE_CMD up -d
  success "Services started"
}

# ---------------------------------------------------------------------------
# IOTP extraction
# ---------------------------------------------------------------------------

wait_for_iotp() {
  header "Waiting for Initial One-Time Password (IOTP)"
  info "The container is starting up. This may take a moment..."

  local elapsed=0
  local iotp=""

  while [ "$elapsed" -lt "$IOTP_MAX_WAIT_SECONDS" ]; do
    iotp="$($COMPOSE_CMD logs 2>&1 \
      | grep -F "$IOTP_GREP_PATTERN" \
      | tail -n1 \
      | sed "s/.*${IOTP_GREP_PATTERN}//" || true)"

    if [ -n "$iotp" ]; then
      display_success "$iotp"
      return 0
    fi

    sleep "$IOTP_POLL_INTERVAL"
    elapsed=$((elapsed + IOTP_POLL_INTERVAL))
    printf '.' >&2
  done

  printf '\n' >&2
  warn "Timed out waiting for IOTP after ${IOTP_MAX_WAIT_SECONDS}s."
  warn "The container may still be starting. Check logs manually:"
  warn "  $COMPOSE_CMD logs | grep -F \"INITIAL ONE-TIME PASSWORD (IOTP): \""
  warn ""
  warn "If a user was already configured, no IOTP is emitted."
  warn "Open $SERVICE_URL and sign in with your existing credentials."
}

display_success() {
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
  printf '  %sUseful commands:%s\n' "$COLOR_BOLD" "$COLOR_RESET"
  printf '    View logs:     %s logs -f\n' "$COMPOSE_CMD"
  printf '    Stop service:  %s down\n' "$COMPOSE_CMD"
  printf '    Restart:       %s restart\n' "$COMPOSE_CMD"
  printf '    Update image:  %s pull && %s up -d\n' "$COMPOSE_CMD" "$COMPOSE_CMD"
  printf '\n'
  printf '  Docs: https://github.com/pRizz/opencode-cloud\n'
  printf '\n'
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
  wait_for_iotp
}

main "$@"
