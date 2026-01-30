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
# Stage 1: Runtime
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
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
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
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    sqlite3=3.45.* \
    postgresql-client=16+* \
    default-mysql-client=1.1.* \
    && rm -rf /var/lib/apt/lists/*

# Group 3: Development libraries for compiling tools (2026-01-22)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    # libssl-dev depends on libssl3t64 with exact version match
    libssl3t64=3.0.* \
    libssl-dev=3.0.* \
    libffi-dev=3.4.* \
    zlib1g-dev=1:1.3.* \
    libbz2-dev=1.0.* \
    libreadline-dev=8.2-* \
    libsqlite3-dev=3.45.* \
    libncurses-dev=6.4+* \
    libpam0g-dev=1.5.* \
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
# Disabled temporarily to reduce Docker build time.
# RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

# Starship prompt - self-managing installer, trusted to handle versions
# Disabled temporarily to reduce Docker build time.
# RUN curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /home/opencode/.local/bin

# Configure zsh with starship
# Disabled temporarily to reduce Docker build time.
# RUN echo 'eval "$(starship init zsh)"' >> /home/opencode/.zshrc \
#     && echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

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
# Uses stable toolchain with minimal profile (skip docs and extras)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
    && . /home/opencode/.cargo/env \
    && rustup default stable \
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
# Use BuildKit cache mount for pip cache
RUN --mount=type=cache,target=/home/opencode/.cache/pip,uid=1000,gid=1000,mode=0755 \
    mkdir -p /home/opencode/.cache/pip \
    && eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pip install --user pipx \
    && pipx ensurepath

# Install global TypeScript compiler
# NOTE: Avoid cache mount here to prevent pnpm store permission issues
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pnpm add -g typescript

# -----------------------------------------------------------------------------
# Modern CLI Tools (Rust-based) - pinned versions (2026-01-22)
# -----------------------------------------------------------------------------
# ripgrep 15.1.0 - fast regex search
# eza 0.23.4 - modern ls replacement
# NOTE: Avoid cache mounts here due to permission issues with non-root Cargo cache
RUN mkdir -p /home/opencode/.cargo/registry /home/opencode/.cargo/git \
    && . /home/opencode/.cargo/env \
    && cargo install --locked ripgrep@15.1.0 eza@0.23.4 \
    && rm -rf /home/opencode/.cargo/registry /home/opencode/.cargo/git

# lazygit v0.58.1 (2026-01-12) - terminal UI for git
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && go install github.com/jesseduffield/lazygit@v0.58.1

# -----------------------------------------------------------------------------
# Additional Development Tools
# -----------------------------------------------------------------------------
# fzf v0.67.0 (2025-11-16) - fuzzy finder
# Disabled temporarily to reduce Docker build time.
# RUN git clone --branch v0.67.0 --depth 1 https://github.com/junegunn/fzf.git /home/opencode/.fzf \
#     && /home/opencode/.fzf/install --all --no-bash --no-fish

# yq v4.50.1 (2025-12-14) - YAML processor
# Disabled temporarily to reduce Docker build time.
# RUN curl -sL https://github.com/mikefarah/yq/releases/download/v4.50.1/yq_linux_$(dpkg --print-architecture) -o /home/opencode/.local/bin/yq \
#     && chmod +x /home/opencode/.local/bin/yq

# Install direnv (2026-01-22)
# Disabled temporarily to reduce Docker build time.
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends direnv=2.32.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencode
# RUN echo 'eval "$(direnv hook zsh)"' >> /home/opencode/.zshrc

# Install HTTPie
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && pipx install httpie

# Install shellcheck (2026-01-22)
# Disabled temporarily to reduce Docker build time.
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends shellcheck=0.9.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencode
# shfmt v3.12.0 (2025-07-06) - shell formatter
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && go install mvdan.cc/sh/v3/cmd/shfmt@v3.12.0

# Install btop system monitor (2026-01-22)
# Use BuildKit cache mount for APT package lists and cache
USER root
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends btop=1.3.* \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# -----------------------------------------------------------------------------
# GitHub CLI
# -----------------------------------------------------------------------------
USER root
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# -----------------------------------------------------------------------------
# Cockpit Web Console (2026-01-22) - temporarily disabled
# -----------------------------------------------------------------------------
# Cockpit provides web-based administration for the container
# Ubuntu noble has cockpit 316 in main repos
# Use BuildKit cache mount for APT package lists and cache
# USER root
# RUN --mount=type=cache,target=/var/lib/apt/lists \
#     --mount=type=cache,target=/var/cache/apt \
#     apt-get update && \
#     apt-get install -y --no-install-recommends \
#     cockpit-ws \
#     cockpit-system \
#     cockpit-bridge \
#     && rm -rf /var/lib/apt/lists/*
#
# Enable Cockpit socket activation (manual symlink since systemctl doesn't work during build)
# RUN mkdir -p /etc/systemd/system/sockets.target.wants \
#     && ln -sf /lib/systemd/system/cockpit.socket /etc/systemd/system/sockets.target.wants/cockpit.socket
#
# Configure Cockpit for HTTP (TLS terminated externally) and proxy headers
# RUN mkdir -p /etc/cockpit && \
#     printf '%s\n' \
#     '[WebService]' \
#     '# Allow HTTP connections (TLS terminated externally like opencode)' \
#     'AllowUnencrypted = true' \
#     '' \
#     '# Trust proxy headers for X-Forwarded-For, X-Forwarded-Proto' \
#     'ProtocolHeader = X-Forwarded-Proto' \
#     'ForwardedForHeader = X-Forwarded-For' \
#     '' \
#     '# Limit concurrent login attempts' \
#     'MaxStartups = 10' \
#     > /etc/cockpit/cockpit.conf
#
# USER opencode

# -----------------------------------------------------------------------------
# CI/CD + tooling (disabled)
# -----------------------------------------------------------------------------
# NOTE: Commented out because this section adds significant time in GitHub Actions
# builds. We will reconsider re-adding these tools later, potentially in a more
# templated/configured Docker image optimized for build tooling.
# -----------------------------------------------------------------------------
# # CI/CD Tools
# # -----------------------------------------------------------------------------
# # act v0.2.84 (2026-01-01) - run GitHub Actions locally
# RUN curl -sL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash -s -- -b /home/opencode/.local/bin v0.2.84
#
# # -----------------------------------------------------------------------------
# # Rust Tooling - pinned versions (2026-01-22)
# # -----------------------------------------------------------------------------
# # cargo-nextest 0.9.122 - fast test runner
# # cargo-audit 0.22.0 - security audit
# # cargo-deny 0.19.0 - dependency linter
# RUN . /home/opencode/.cargo/env \
#     && cargo install --locked cargo-nextest@0.9.122 cargo-audit@0.22.0 cargo-deny@0.19.0
#
# # Install mold fast linker (2026-01-22)
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends mold=2.30.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencode
#
# # -----------------------------------------------------------------------------
# # Code Quality Tools
# # -----------------------------------------------------------------------------
# # JavaScript/TypeScript tools
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && pnpm add -g \
#     prettier \
#     eslint \
#     @biomejs/biome
#
# # Python tools
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && pipx install black \
#     && pipx install ruff
#
# # Test runners (commonly needed)
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && pnpm add -g jest vitest
#
# # Python pytest via pipx
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && pipx install pytest
#
# # -----------------------------------------------------------------------------
# # Protocol Buffers / gRPC (2026-01-22)
# # -----------------------------------------------------------------------------
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler=3.21.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencode
#
# # grpcurl v1.9.3 (2025-03-11) - gRPC debugging tool
# RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
#     && go install github.com/fullstorydev/grpcurl/cmd/grpcurl@v1.9.3

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
    && printf '%s\n' \
    '# Minimal starship config for fast prompt' \
    'format = """' \
    '$directory\' \
    '$git_branch\' \
    '$git_status\' \
    '$character"""' \
    '' \
    '[directory]' \
    'truncation_length = 3' \
    'truncate_to_repo = true' \
    '' \
    '[git_branch]' \
    'format = "[$branch]($style) "' \
    'style = "bold purple"' \
    '' \
    '[git_status]' \
    'format = '"'"'([$all_status$ahead_behind]($style) )'"'"'' \
    '' \
    '[character]' \
    'success_symbol = "[>](bold green)"' \
    'error_symbol = "[>](bold red)"' \
    > /home/opencode/.config/starship.toml

# Shell aliases
RUN printf '%s\n' \
    '' \
    '# Modern CLI aliases' \
    'alias ls="eza --icons"' \
    'alias ll="eza -l --icons"' \
    'alias la="eza -la --icons"' \
    'alias lt="eza --tree --icons"' \
    'alias grep="rg"' \
    'alias top="btop"' \
    '' \
    '# Git aliases' \
    'alias g="git"' \
    'alias gs="git status"' \
    'alias gd="git diff"' \
    'alias gc="git commit"' \
    'alias gp="git push"' \
    'alias gl="git pull"' \
    'alias gco="git checkout"' \
    'alias gb="git branch"' \
    'alias lg="lazygit"' \
    '' \
    '# Docker aliases (for Docker-in-Docker)' \
    'alias d="docker"' \
    'alias dc="docker compose"' \
    '' \
    >> /home/opencode/.zshrc

# Set up pipx path
RUN echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

# -----------------------------------------------------------------------------
# opencode Setup (Fork + Broker + Proxy)
# -----------------------------------------------------------------------------
# Keep this block near the end to preserve cache for earlier layers. We expect
# opencode fork changes to happen more often than base tooling changes.
#
# This block includes:
# - opencode build (backend binary) + app build (frontend dist)
# - opencode-broker build
# - opencode web build + runtime
# - PAM configuration + systemd services
# - opencode config file
#
# NOTE: This section switches between opencode and root users as needed.

# -----------------------------------------------------------------------------
# opencode Installation (Fork from pRizz/opencode)
# -----------------------------------------------------------------------------
# Clone the fork and build opencode from source (as non-root user)
# Pin to specific commit for reproducibility
# Build opencode from source (BuildKit cache mounts disabled for now)
RUN OPENCODE_COMMIT="c7fb116c1cf59a76b184f842ed3eb0113d93196b" \
    && rm -rf /tmp/opencode-repo \
    && git clone --depth 1 https://github.com/pRizz/opencode.git /tmp/opencode-repo \
    && cd /tmp/opencode-repo \
    && git fetch --depth 1 origin "${OPENCODE_COMMIT}" \
    && git checkout "${OPENCODE_COMMIT}" \
    && curl -fsSL https://bun.sh/install | bash -s "bun-v1.3.5" \
    && export PATH="/home/opencode/.bun/bin:${PATH}" \
    && bun install --frozen-lockfile \
    && cd packages/opencode \
    && export VITE_OPENCODE_SERVER_URL="http://localhost:3000" \
    && bun run build-single-ui \
    && rm -rf /home/opencode/.bun/install/cache /home/opencode/.bun/cache /home/opencode/.cache/bun \
    && cd /tmp/opencode-repo \
    && mkdir -p /home/opencode/.local/share/opencode/bin \
    && mkdir -p /home/opencode/.local/share/opencode/ui \
    && cp /tmp/opencode-repo/packages/opencode/dist/opencode-*/bin/opencode /home/opencode/.local/share/opencode/bin/opencode \
    && cp -R /tmp/opencode-repo/packages/opencode/dist/opencode-*/ui/. /home/opencode/.local/share/opencode/ui/ \
    && chown -R opencode:opencode /home/opencode/.local/share/opencode \
    && chmod +x /home/opencode/.local/share/opencode/bin/opencode \
    && /home/opencode/.local/share/opencode/bin/opencode --version

# Add opencode to PATH
ENV PATH="/home/opencode/.local/share/opencode/bin:${PATH}"

# -----------------------------------------------------------------------------
# opencode-broker Installation
# -----------------------------------------------------------------------------
# Build opencode-broker from source (Rust service for PAM authentication)
# The broker handles PAM authentication and user process spawning
# NOTE: Requires root privileges for setuid bit (chmod 4755) to allow broker
# to run with elevated privileges for PAM authentication
USER root
RUN cd /tmp/opencode-repo/packages/opencode-broker \
    && runuser -u opencode -- bash -c '. /home/opencode/.cargo/env && cargo build --release' \
    && mkdir -p /usr/local/bin \
    && cp target/release/opencode-broker /usr/local/bin/opencode-broker \
    && chmod 4755 /usr/local/bin/opencode-broker \
    && rm -rf /tmp/opencode-repo

# Verify broker binary exists and is executable
RUN ls -la /usr/local/bin/opencode-broker \
    && test -x /usr/local/bin/opencode-broker \
    && echo "Broker installed"

# -----------------------------------------------------------------------------
# PAM Configuration
# -----------------------------------------------------------------------------
# Install PAM configuration for opencode authentication
# This allows opencode to authenticate users via PAM (same users as Cockpit)
# NOTE: Requires root privileges to write to /etc/pam.d/
RUN printf '%s\n' \
    '# PAM configuration for OpenCode authentication' \
    '# Install to /etc/pam.d/opencode' \
    '' \
    '# Standard UNIX authentication' \
    'auth required pam_unix.so' \
    'account required pam_unix.so' \
    '' \
    '# Optional: Enable TOTP 2FA (uncomment when pam_google_authenticator is installed)' \
    '# auth required pam_google_authenticator.so' \
    > /etc/pam.d/opencode \
    && chmod 644 /etc/pam.d/opencode

# Verify PAM config file exists
RUN ls -la /etc/pam.d/opencode && cat /etc/pam.d/opencode

# -----------------------------------------------------------------------------
# opencode-broker systemd Service
# -----------------------------------------------------------------------------
# Create opencode-broker service for PAM authentication
# NOTE: Requires root privileges to write to /etc/systemd/system/
RUN printf '%s\n' \
    '[Unit]' \
    'Description=OpenCode Authentication Broker' \
    'Documentation=https://github.com/pRizz/opencode' \
    'After=network.target' \
    '' \
    '[Service]' \
    'Type=notify' \
    'ExecStart=/usr/local/bin/opencode-broker' \
    'ExecReload=/bin/kill -HUP $MAINPID' \
    'Restart=always' \
    'RestartSec=5' \
    '' \
    '# Security hardening' \
    'NoNewPrivileges=false' \
    'ProtectSystem=strict' \
    'ProtectHome=read-only' \
    'PrivateTmp=true' \
    'ReadWritePaths=/run/opencode' \
    '' \
    '# Socket directory' \
    'RuntimeDirectory=opencode' \
    'RuntimeDirectoryMode=0755' \
    '' \
    '# Logging' \
    'StandardOutput=journal' \
    'StandardError=journal' \
    'SyslogIdentifier=opencode-broker' \
    '' \
    '[Install]' \
    'WantedBy=multi-user.target' \
    > /etc/systemd/system/opencode-broker.service

# Enable opencode-broker service
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode-broker.service /etc/systemd/system/multi-user.target.wants/opencode-broker.service

USER opencode

# -----------------------------------------------------------------------------
# GSD Plugin Installation
# -----------------------------------------------------------------------------
# Install the GSD (Get Shit Done) plugin for opencode
# Note: If this fails in container builds due to "~" path resolution, retry with
# OPENCODE_CONFIG_DIR=/home/opencode/.config/opencode set explicitly.
RUN mkdir -p /home/opencode/.npm \
    && npx --yes get-shit-done-cc --opencode --global \
    && rm -rf /home/opencode/.npm/_cacache /home/opencode/.npm/_npx

# -----------------------------------------------------------------------------
# opencode systemd Service (2026-01-22)
# -----------------------------------------------------------------------------
# Create opencode as a systemd service for Cockpit integration
# NOTE: Requires root privileges to write to /etc/systemd/system/
USER root
RUN printf '%s\n' \
    '[Unit]' \
    'Description=opencode Web Interface' \
    'After=network.target opencode-broker.service' \
    '' \
    '[Service]' \
    'Type=simple' \
    'User=opencode' \
    'WorkingDirectory=/home/opencode/workspace' \
    'ExecStart=/home/opencode/.local/share/opencode/bin/opencode web --port 3000 --hostname 0.0.0.0' \
    'Restart=always' \
    'RestartSec=5' \
    'Environment=PATH=/home/opencode/.local/share/opencode/bin:/home/opencode/.local/bin:/home/opencode/.cargo/bin:/home/opencode/.local/share/mise/shims:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin' \
    '' \
    '[Install]' \
    'WantedBy=multi-user.target' \
    > /etc/systemd/system/opencode.service

# Enable opencode service to start at boot (manual symlink since systemctl doesn't work during build)
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode.service /etc/systemd/system/multi-user.target.wants/opencode.service

# -----------------------------------------------------------------------------
# opencode Configuration
# -----------------------------------------------------------------------------
# Create opencode.jsonc config file with PAM authentication enabled
RUN mkdir -p /home/opencode/.config/opencode \
    && printf '%s\n' \
    '{' \
    '  "auth": {' \
    '    "enabled": true' \
    '  }' \
    '}' \
    > /home/opencode/.config/opencode/opencode.jsonc \
    && chown -R opencode:opencode /home/opencode/.config/opencode \
    && chmod 644 /home/opencode/.config/opencode/opencode.jsonc

# Verify config file exists
RUN ls -la /home/opencode/.config/opencode/opencode.jsonc && cat /home/opencode/.config/opencode/opencode.jsonc

USER opencode

# -----------------------------------------------------------------------------
# Entrypoint Script (Hybrid Init Support)
# -----------------------------------------------------------------------------
# Supports both tini (default, works everywhere) and systemd (for Cockpit on Linux)
# Set USE_SYSTEMD=1 environment variable to use systemd init
# Note: Entrypoint runs as root to support both modes; tini mode drops to opencode user
USER root
RUN printf '%s\n' \
    '#!/bin/bash' \
    'if [ "${USE_SYSTEMD}" = "1" ]; then' \
    '    exec /sbin/init' \
    'else' \
    '    # Ensure broker socket directory exists' \
    '    install -d -m 0755 /run/opencode' \
    '    /usr/local/bin/opencode-broker &' \
    '    # Use runuser to switch to opencode user without password prompt' \
    '    exec runuser -u opencode -- sh -lc "cd /home/opencode/workspace && /home/opencode/.local/share/opencode/bin/opencode web --port 3000 --hostname 0.0.0.0"' \
    'fi' \
    > /usr/local/bin/entrypoint.sh && chmod +x /usr/local/bin/entrypoint.sh

# Note: Don't set USER here - entrypoint needs root to use runuser
# The tini mode drops privileges to opencode user via runuser

# -----------------------------------------------------------------------------
# Health Check
# -----------------------------------------------------------------------------
# Check that opencode main page responds
# Works for both tini and systemd modes
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f -H "Accept: text/html" http://localhost:3000/ || exit 1

# -----------------------------------------------------------------------------
# Version File
# -----------------------------------------------------------------------------
# Store version in file for runtime access (debugging, scripts)
# Keep this near the end: changing the version invalidates the cache
# and significantly slows down docker builds if placed earlier.
USER root
ARG OPENCODE_CLOUD_VERSION=dev
LABEL org.opencode-cloud.version="${OPENCODE_CLOUD_VERSION}"
RUN echo "${OPENCODE_CLOUD_VERSION}" > /etc/opencode-cloud-version
# Note: Stay as root - entrypoint.sh needs root to run either:
# - /sbin/init (systemd mode)
# - runuser (tini mode, which then drops privileges to opencode user)

# -----------------------------------------------------------------------------
# Final Configuration
# -----------------------------------------------------------------------------
WORKDIR /home/opencode/workspace

# Expose opencode web (3000) and Cockpit (9090)
EXPOSE 3000 9090

# Hybrid init: entrypoint script chooses tini or systemd based on USE_SYSTEMD env
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
