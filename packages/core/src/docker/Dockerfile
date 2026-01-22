# =============================================================================
# opencode-cloud Container Image
# =============================================================================
# A comprehensive development environment for AI-assisted coding with opencode.
#
# Features:
# - Ubuntu 24.04 LTS (noble) base
# - Non-root user with passwordless sudo
# - Multiple languages via mise (Node.js, Python, Rust, Go)
# - Modern CLI tools (ripgrep, eza, fzf, lazygit, etc.)
# - GSD opencode plugin for task management
#
# Usage:
#   docker build -t opencode-cloud .
#   docker run -it opencode-cloud
#
# =============================================================================

# -----------------------------------------------------------------------------
# Version Pinning Policy
# -----------------------------------------------------------------------------
# - APT packages: Use major.minor.* wildcards for patch updates
# - GitHub tools: Pin to release tags (vX.Y.Z)
# - Cargo/Go: Pin to exact versions (@X.Y.Z)
# - Security exceptions marked with: # UNPINNED: package - reason
# - Self-managing installers (mise, rustup, etc.) trusted to handle versions
#
# To check for updates: just check-updates
# Last version audit: 2026-01-22
# -----------------------------------------------------------------------------

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
# Install build dependencies and tools that require compilation
FROM ubuntu:24.04 AS builder

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install build essentials (2026-01-22)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential=12.* \
    # UNPINNED: ca-certificates - security-critical root certs, needs auto-updates
    ca-certificates \
    curl=8.5.* \
    git=1:2.43.* \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM ubuntu:24.04 AS runtime

# OCI Labels for image metadata
LABEL org.opencontainers.image.title="opencode-cloud"
LABEL org.opencontainers.image.description="AI-assisted development environment with opencode"
LABEL org.opencontainers.image.url="https://github.com/pRizz/opencode-cloud"
LABEL org.opencontainers.image.source="https://github.com/pRizz/opencode-cloud"
LABEL org.opencontainers.image.vendor="pRizz"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.base.name="ubuntu:24.04"

# Environment configuration
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# -----------------------------------------------------------------------------
# System Dependencies
# -----------------------------------------------------------------------------
# Install core system packages in logical groups for better caching

# Group 1: Core utilities and build tools (2026-01-22)
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Init systems
    tini=0.19.* \
    dumb-init=1.2.* \
    # systemd for Cockpit support
    systemd=255.* \
    systemd-sysv=255.* \
    dbus=1.14.* \
    # Shell and terminal
    zsh=5.9-* \
    tmux=3.4-* \
    # Editors
    vim=2:9.1.* \
    neovim=0.9.* \
    nano=7.2-* \
    # Build essentials
    build-essential=12.* \
    pkg-config=1.8.* \
    cmake=3.28.* \
    # Version control
    git=1:2.43.* \
    git-lfs=3.4.* \
    # Core utilities
    curl=8.5.* \
    wget=1.21.* \
    # UNPINNED: ca-certificates - security-critical root certs, needs auto-updates
    ca-certificates \
    # UNPINNED: gnupg - key management security, needs auto-updates
    gnupg \
    lsb-release=12.* \
    software-properties-common=0.99.* \
    sudo=1.9.* \
    # UNPINNED: openssh-client - security-critical, needs auto-updates
    openssh-client \
    # Process/system tools
    htop=3.3.* \
    procps=2:4.0.* \
    less=590-* \
    file=1:5.45-* \
    tree=2.1.* \
    # JSON/YAML processing
    jq=1.7.* \
    # Network tools
    netcat-openbsd=1.226-* \
    iputils-ping=3:20240117-* \
    dnsutils=1:9.18.* \
    # Compression
    zip=3.0-* \
    unzip=6.0-* \
    xz-utils=5.6.* \
    p7zip-full=16.02* \
    && rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services for container environment
RUN systemctl mask \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount \
    systemd-update-utmp.service \
    systemd-tmpfiles-setup.service \
    systemd-remount-fs.service

# Group 2: Database clients (2026-01-22)
RUN apt-get update && apt-get install -y --no-install-recommends \
    sqlite3=3.45.* \
    postgresql-client=16+* \
    default-mysql-client=1.1.* \
    && rm -rf /var/lib/apt/lists/*

# Group 3: Development libraries for compiling tools (2026-01-22)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev=3.0.* \
    libffi-dev=3.4.* \
    zlib1g-dev=1:1.3.* \
    libbz2-dev=1.0.* \
    libreadline-dev=8.2-* \
    libsqlite3-dev=3.45.* \
    libncurses-dev=6.4+* \
    liblzma-dev=5.6.* \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Create Non-Root User
# -----------------------------------------------------------------------------
# Create 'opencode' user with passwordless sudo
RUN useradd -m -s /bin/zsh -G sudo opencode \
    && echo "opencode ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/opencode \
    && chmod 0440 /etc/sudoers.d/opencode

# Switch to opencode user for remaining setup
USER opencode
WORKDIR /home/opencode

# Set up directories
RUN mkdir -p \
    /home/opencode/.config \
    /home/opencode/.local/bin \
    /home/opencode/.local/share \
    /home/opencode/.cache \
    /home/opencode/workspace

# Add local bin to PATH
ENV PATH="/home/opencode/.local/bin:${PATH}"

# -----------------------------------------------------------------------------
# Shell Setup: Zsh + Oh My Zsh + Starship
# -----------------------------------------------------------------------------
# Oh My Zsh - self-managing installer, trusted to handle versions
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

# Starship prompt - self-managing installer, trusted to handle versions
RUN curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /home/opencode/.local/bin

# Configure zsh with starship
RUN echo 'eval "$(starship init zsh)"' >> /home/opencode/.zshrc \
    && echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

# -----------------------------------------------------------------------------
# mise: Universal Version Manager
# -----------------------------------------------------------------------------
# mise - self-managing installer, trusted to handle versions
RUN curl https://mise.run | sh \
    && echo 'eval "$(/home/opencode/.local/bin/mise activate zsh)"' >> /home/opencode/.zshrc

# Install language runtimes via mise (2026-01-22)
# - node@lts: mise handles LTS resolution (currently 22.x)
# - python@3.12: pinned to minor version
# - go@1.24: pinned to minor version (was @latest)
RUN /home/opencode/.local/bin/mise install node@lts \
    && /home/opencode/.local/bin/mise install python@3.12 \
    && /home/opencode/.local/bin/mise install go@1.24 \
    && /home/opencode/.local/bin/mise use --global node@lts \
    && /home/opencode/.local/bin/mise use --global python@3.12 \
    && /home/opencode/.local/bin/mise use --global go@1.24

# Set up mise shims in PATH for non-interactive shells
ENV PATH="/home/opencode/.local/share/mise/shims:${PATH}"

# -----------------------------------------------------------------------------
# Rust Installation
# -----------------------------------------------------------------------------
# rustup - self-managing installer, trusted to handle versions
# Uses stable toolchain (rustup manages toolchain versioning)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && . /home/opencode/.cargo/env \
    && rustup component add rust-analyzer rustfmt clippy

ENV PATH="/home/opencode/.cargo/bin:${PATH}"

# -----------------------------------------------------------------------------
# Package Managers
# -----------------------------------------------------------------------------
# Switch to bash for mise activation (mise outputs bash-specific syntax)
SHELL ["/bin/bash", "-c"]

# Install pnpm 10.x via corepack (2026-01-22)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && corepack enable \
    && corepack prepare pnpm@10.28.1 --activate

# Set up pnpm global bin directory
ENV PNPM_HOME="/home/opencode/.local/share/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"
RUN mkdir -p "${PNPM_HOME}"

# uv - self-managing installer, trusted to handle versions (fast Python package manager)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install pipx for isolated Python application installs
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pip install --user pipx \
    && pipx ensurepath

# Install global TypeScript compiler
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pnpm add -g typescript

# -----------------------------------------------------------------------------
# Modern CLI Tools (Rust-based) - pinned versions (2026-01-22)
# -----------------------------------------------------------------------------
# ripgrep 15.1.0 - fast regex search
# eza 0.23.4 - modern ls replacement
RUN . /home/opencode/.cargo/env \
    && cargo install --locked ripgrep@15.1.0 eza@0.23.4

# lazygit v0.58.1 (2026-01-12) - terminal UI for git
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install github.com/jesseduffield/lazygit@v0.58.1

# -----------------------------------------------------------------------------
# Additional Development Tools
# -----------------------------------------------------------------------------
# fzf v0.67.0 (2025-11-16) - fuzzy finder
RUN git clone --branch v0.67.0 --depth 1 https://github.com/junegunn/fzf.git /home/opencode/.fzf \
    && /home/opencode/.fzf/install --all --no-bash --no-fish

# yq v4.50.1 (2025-12-14) - YAML processor
RUN curl -sL https://github.com/mikefarah/yq/releases/download/v4.50.1/yq_linux_$(dpkg --print-architecture) -o /home/opencode/.local/bin/yq \
    && chmod +x /home/opencode/.local/bin/yq

# Install direnv (2026-01-22)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends direnv=2.32.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode
RUN echo 'eval "$(direnv hook zsh)"' >> /home/opencode/.zshrc

# Install HTTPie
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pipx install httpie

# Install shellcheck (2026-01-22)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends shellcheck=0.9.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode
# shfmt v3.12.0 (2025-07-06) - shell formatter
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install mvdan.cc/sh/v3/cmd/shfmt@v3.12.0

# Install btop system monitor (2026-01-22)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends btop=1.3.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# -----------------------------------------------------------------------------
# GitHub CLI
# -----------------------------------------------------------------------------
USER root
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# -----------------------------------------------------------------------------
# CI/CD Tools
# -----------------------------------------------------------------------------
# act v0.2.84 (2026-01-01) - run GitHub Actions locally
RUN curl -sL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash -s -- -b /home/opencode/.local/bin v0.2.84

# -----------------------------------------------------------------------------
# Rust Tooling - pinned versions (2026-01-22)
# -----------------------------------------------------------------------------
# cargo-nextest 0.9.122 - fast test runner
# cargo-audit 0.22.0 - security audit
# cargo-deny 0.19.0 - dependency linter
RUN . /home/opencode/.cargo/env \
    && cargo install --locked cargo-nextest@0.9.122 cargo-audit@0.22.0 cargo-deny@0.19.0

# Install mold fast linker (2026-01-22)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends mold=2.30.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# -----------------------------------------------------------------------------
# Code Quality Tools
# -----------------------------------------------------------------------------
# JavaScript/TypeScript tools
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pnpm add -g \
    prettier \
    eslint \
    @biomejs/biome

# Python tools
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pipx install black \
    && pipx install ruff

# Test runners (commonly needed)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pnpm add -g jest vitest

# Python pytest via pipx
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pipx install pytest

# -----------------------------------------------------------------------------
# Protocol Buffers / gRPC (2026-01-22)
# -----------------------------------------------------------------------------
USER root
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler=3.21.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# grpcurl v1.9.3 (2025-03-11) - gRPC debugging tool
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install github.com/fullstorydev/grpcurl/cmd/grpcurl@v1.9.3

# -----------------------------------------------------------------------------
# opencode Installation
# -----------------------------------------------------------------------------
# opencode - self-managing installer, trusted to handle versions
# The script installs to ~/.opencode/bin/
RUN curl -fsSL https://opencode.ai/install | bash \
    && ls -la /home/opencode/.opencode/bin/opencode \
    && /home/opencode/.opencode/bin/opencode --version

# Add opencode to PATH
ENV PATH="/home/opencode/.opencode/bin:${PATH}"

# -----------------------------------------------------------------------------
# GSD Plugin Installation
# -----------------------------------------------------------------------------
# Install the GSD (Get Shit Done) plugin for opencode
RUN git clone https://github.com/rokicool/gsd-opencode.git /home/opencode/.config/opencode/plugins/gsd-opencode

# -----------------------------------------------------------------------------
# Sensible Defaults Configuration
# -----------------------------------------------------------------------------
# Git configuration
RUN git config --global init.defaultBranch main \
    && git config --global core.editor "nvim" \
    && git config --global pull.rebase false \
    && git config --global merge.conflictstyle diff3 \
    && git config --global diff.colorMoved default

# Starship configuration (minimal, fast prompt)
RUN mkdir -p /home/opencode/.config \
    && cat > /home/opencode/.config/starship.toml << 'STARSHIP'
# Minimal starship config for fast prompt
format = """
$directory\
$git_branch\
$git_status\
$character"""

[directory]
truncation_length = 3
truncate_to_repo = true

[git_branch]
format = "[$branch]($style) "
style = "bold purple"

[git_status]
format = '([$all_status$ahead_behind]($style) )'

[character]
success_symbol = "[>](bold green)"
error_symbol = "[>](bold red)"
STARSHIP

# Shell aliases
RUN cat >> /home/opencode/.zshrc << 'ALIASES'

# Modern CLI aliases
alias ls="eza --icons"
alias ll="eza -l --icons"
alias la="eza -la --icons"
alias lt="eza --tree --icons"
alias grep="rg"
alias top="btop"

# Git aliases
alias g="git"
alias gs="git status"
alias gd="git diff"
alias gc="git commit"
alias gp="git push"
alias gl="git pull"
alias gco="git checkout"
alias gb="git branch"
alias lg="lazygit"

# Docker aliases (for Docker-in-Docker)
alias d="docker"
alias dc="docker compose"

ALIASES

# Set up pipx path
RUN echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

# -----------------------------------------------------------------------------
# Health Check
# -----------------------------------------------------------------------------
# Port 3000 must match OPENCODE_WEB_PORT in container.rs
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# -----------------------------------------------------------------------------
# Final Configuration
# -----------------------------------------------------------------------------
WORKDIR /home/opencode/workspace

# Expose opencode web port (matches OPENCODE_WEB_PORT in container.rs)
EXPOSE 3000

# Use tini as init system for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command: start opencode web interface
# - Port 3000 must match OPENCODE_WEB_PORT in container.rs
# - Using full path to ensure binary is found (opencode installs to ~/.opencode/bin/)
# - --hostname 0.0.0.0 is required for Docker port mapping (default 127.0.0.1 only listens on loopback)
CMD ["/home/opencode/.opencode/bin/opencode", "web", "--port", "3000", "--hostname", "0.0.0.0"]
