# =============================================================================
# opencode-cloud Container Image
# =============================================================================
# A comprehensive development environment for AI-assisted coding with opencode.
#
# Features:
# - Ubuntu 25.10 (questing) base
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
# Last version audit: 2026-02-03
# -----------------------------------------------------------------------------

# -----------------------------------------------------------------------------
# Stage 1: Base
# -----------------------------------------------------------------------------
FROM ubuntu:25.10 AS base

# OCI Labels for image metadata
LABEL org.opencontainers.image.title="opencode-cloud-sandbox"
# NOTE: This exact label format is parsed by scripts/extract-oci-description.py
# (called from .github/workflows/docker-publish.yml) to populate multi-arch
# manifest annotations for GHCR. If you change this line (format, quoting, or
# key), update the extraction logic too.
LABEL org.opencontainers.image.description="AI-assisted development environment with opencode"
LABEL org.opencontainers.image.url="https://github.com/pRizz/opencode-cloud"
LABEL org.opencontainers.image.source="https://github.com/pRizz/opencode-cloud"
LABEL org.opencontainers.image.vendor="pRizz"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.base.name="ubuntu:25.10"

# Environment configuration
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# -----------------------------------------------------------------------------
# System Dependencies
# -----------------------------------------------------------------------------
# Install core system packages in logical groups for better caching

# Group 1: Core utilities and build tools (2026-02-03)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    # Init systems
    tini=0.19.* \
    dumb-init=1.2.* \
    # systemd for Cockpit support
    systemd=257.* \
    systemd-sysv=257.* \
    dbus=1.16.* \
    # Shell and terminal
    tmux=3.5a-* \
    # Editors
    vim=2:9.1.* \
    neovim=0.10.* \
    nano=8.4-* \
    # Build essentials
    build-essential=12.* \
    pkg-config=1.8.* \
    cmake=3.31.* \
    # Version control
    git=1:2.51.* \
    git-lfs=3.6.* \
    # Core utilities
    curl=8.14.* \
    wget=1.25.* \
    # UNPINNED: ca-certificates - security-critical root certs, needs auto-updates
    ca-certificates \
    # UNPINNED: gnupg - key management security, needs auto-updates
    gnupg \
    lsb-release=12.* \
    software-properties-common=0.114* \
    sudo=1.9.* \
    # UNPINNED: openssh-client - security-critical, needs auto-updates
    openssh-client \
    # Process/system tools
    htop=3.4.* \
    procps=2:4.0.* \
    less=668-* \
    file=1:5.46-* \
    tree=2.2.* \
    # JSON/YAML processing
    jq=1.8.* \
    # Network tools
    netcat-openbsd=1.229-* \
    iputils-ping=3:20240905-* \
    bind9-dnsutils=1:9.20.* \
    # Compression
    zip=3.0-* \
    unzip=6.0-* \
    xz-utils=5.8.* \
    p7zip-full=16.02* \
    && rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services for container environment
RUN systemctl mask \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount \
    systemd-update-utmp.service \
    systemd-tmpfiles-setup.service \
    systemd-remount-fs.service

# Group 2: Database clients (2026-02-03)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    sqlite3=3.46.* \
    postgresql-client=17+* \
    default-mysql-client=1.1.* \
    && rm -rf /var/lib/apt/lists/*

# Group 3: Development libraries for compiling tools (2026-02-03)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    # libssl-dev depends on libssl3t64 with exact version match
    libssl3t64=3.5.* \
    libssl-dev=3.5.* \
    libffi-dev=3.5.* \
    zlib1g-dev=1:1.3.* \
    libbz2-dev=1.0.* \
    libreadline-dev=8.3-* \
    libsqlite3-dev=3.46.* \
    libncurses-dev=6.5+* \
    libpam0g-dev=1.7.* \
    liblzma-dev=5.8.* \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Create Non-Root User
# -----------------------------------------------------------------------------
# Create 'opencode' user with passwordless sudo
RUN useradd -m -s /bin/bash -G sudo opencode \
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
    /home/opencode/.local/state \
    /home/opencode/.cache \
    /home/opencode/workspace

# Add local bin to PATH
ENV PATH="/home/opencode/.local/bin:${PATH}"

# -----------------------------------------------------------------------------
# Shell Setup: Bash + Starship
# -----------------------------------------------------------------------------
# Starship prompt - self-managing installer, trusted to handle versions
# Disabled temporarily to reduce Docker build time.
# RUN curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /home/opencode/.local/bin

# Configure bash with starship
# Disabled temporarily to reduce Docker build time.
# RUN echo 'eval "$(starship init bash)"' >> /home/opencode/.bashrc \
#     && echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.bashrc

# -----------------------------------------------------------------------------
# mise: Universal Version Manager
# -----------------------------------------------------------------------------
# mise - self-managing installer, trusted to handle versions
RUN curl https://mise.run | sh \
    && echo 'eval "$(/home/opencode/.local/bin/mise activate bash)"' >> /home/opencode/.bashrc

# Install language runtimes via mise (2026-02-03)
# - node@25: pinned to major version
# - python@3.13: pinned to major version (3.13 has precompiled binaries available)
# - go@1.25: pinned to minor version
#
# MISE_PYTHON_COMPILE=0: Disable Python compilation from source.
# If no precompiled binary exists, mise will fail fast instead of hanging
# during a 30+ minute compilation that may timeout in CI.
#
# Timeout: 10 minutes max for all runtime installations as safety net.
RUN timeout 600 sh -c '\
    export MISE_PYTHON_COMPILE=0 && \
    /home/opencode/.local/bin/mise install node@25 && \
    /home/opencode/.local/bin/mise install python@3.13 && \
    /home/opencode/.local/bin/mise install go@1.25 && \
    /home/opencode/.local/bin/mise use --global node@25 && \
    /home/opencode/.local/bin/mise use --global python@3.13 && \
    /home/opencode/.local/bin/mise use --global go@1.25'

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

# Install pnpm 10.x via corepack (2026-02-03)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && npm install -g corepack \
    && corepack enable \
    && corepack prepare pnpm@10.28.2 --activate

# Set up pnpm global bin directory
ENV PNPM_HOME="/home/opencode/.local/share/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"
RUN mkdir -p "${PNPM_HOME}"

# bun - self-managing installer, pinned to version (2026-02-03)
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.3.5" \
    && rm -rf /home/opencode/.bun/install/cache /home/opencode/.bun/cache /home/opencode/.cache/bun

ENV PATH="/home/opencode/.bun/bin:${PATH}"

# uv - pinned installer (fast Python package manager) (2026-02-03)
RUN curl -LsSf https://github.com/astral-sh/uv/releases/download/0.9.21/uv-installer.sh | sh

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
    && pnpm add -g typescript@5.9.2

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
# RUN curl -sL https://github.com/mikefarah/yq/releases/download/v4.52.2/yq_linux_$(dpkg --print-architecture) -o /home/opencode/.local/bin/yq \
#     && chmod +x /home/opencode/.local/bin/yq

# Install direnv (2026-01-22)
# Disabled temporarily to reduce Docker build time.
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends direnv=2.32.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencode
# RUN echo 'eval "$(direnv hook bash)"' >> /home/opencode/.bashrc

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

# Install btop system monitor (2026-02-03)
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
# Ubuntu questing has cockpit 346 in main repos
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
#     && cargo install --locked cargo-nextest@0.9.124 cargo-audit@0.22.0 cargo-deny@0.19.0
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
COPY --chown=opencode:opencode packages/core/src/docker/files/starship.toml /home/opencode/.config/starship.toml

# Shell aliases
COPY --chown=opencode:opencode packages/core/src/docker/files/bashrc.extra /home/opencode/.bashrc.extra
RUN cat /home/opencode/.bashrc.extra >> /home/opencode/.bashrc \
    && rm /home/opencode/.bashrc.extra

# -----------------------------------------------------------------------------
# Stage 2: opencode build
# -----------------------------------------------------------------------------
FROM base AS opencode-build

# -----------------------------------------------------------------------------
# opencode Setup (Fork + Broker + Proxy)
# -----------------------------------------------------------------------------
# Keep this block near the end to preserve cache for earlier layers. We expect
# opencode fork changes to happen more often than base tooling changes.
#
# This block includes:
# - opencode build (backend binary) + app build (frontend dist)
# - opencode-broker build
#
# NOTE: This stage uses opencode user + sudo for privileged installs.
USER opencode

# -----------------------------------------------------------------------------
# opencode Installation (Fork from pRizz/opencode)
# -----------------------------------------------------------------------------
# Clone the fork and build opencode from source (as non-root user)
# Pin to specific commit for reproducibility
# NOTE: OPENCODE_COMMIT is not tied to releases/tags; it tracks the latest stable
# commit on the main branch of https://github.com/pRizz/opencode.
# Update it by running: ./scripts/update-opencode-commit.sh
# Build opencode from source (BuildKit cache mounts disabled for now)
RUN OPENCODE_COMMIT="41731edead75a52aceac0b48c22474bdeafc746c" \
    && rm -rf /tmp/opencode-repo \
    && git clone --depth 1 https://github.com/pRizz/opencode.git /tmp/opencode-repo \
    && cd /tmp/opencode-repo \
    && git fetch --depth 1 origin "${OPENCODE_COMMIT}" \
    && git checkout "${OPENCODE_COMMIT}" \
    && bun install --frozen-lockfile \
    && cd packages/opencode \
    && bun run build-single-ui \
    && cd /tmp/opencode-repo \
    && sudo mkdir -p /opt/opencode/bin /opt/opencode/ui \
    && echo "${OPENCODE_COMMIT}" | sudo tee /opt/opencode/COMMIT >/dev/null \
    && sudo chown opencode:opencode /opt/opencode/COMMIT \
    && sudo cp /tmp/opencode-repo/packages/opencode/dist/opencode-*/bin/opencode /opt/opencode/bin/opencode \
    && sudo cp -R /tmp/opencode-repo/packages/opencode/dist/opencode-*/ui/. /opt/opencode/ui/ \
    && sudo chown -R opencode:opencode /opt/opencode \
    && sudo chmod +x /opt/opencode/bin/opencode \
    && cd /tmp/opencode-repo/packages/opencode-broker \
    && cargo build --release \
    && sudo mkdir -p /usr/local/bin \
    && sudo cp target/release/opencode-broker /usr/local/bin/opencode-broker \
    && sudo chmod 4755 /usr/local/bin/opencode-broker \
    && rm -rf /tmp/opencode-repo \
    && rm -rf /home/opencode/.bun/install/cache /home/opencode/.bun/cache /home/opencode/.cache/bun

# -----------------------------------------------------------------------------
# Stage 3: Runtime
# -----------------------------------------------------------------------------
FROM base AS runtime

# Add opencode to PATH
ENV PATH="/opt/opencode/bin:${PATH}"

# -----------------------------------------------------------------------------
# PAM Configuration
# -----------------------------------------------------------------------------
# Install PAM configuration for opencode authentication
# This allows opencode to authenticate users via PAM (same users as Cockpit)
# NOTE: Requires root privileges to write to /etc/pam.d/
USER root
COPY packages/core/src/docker/files/pam/opencode /etc/pam.d/opencode
RUN chmod 644 /etc/pam.d/opencode

# Verify PAM config file exists
RUN ls -la /etc/pam.d/opencode && cat /etc/pam.d/opencode

# -----------------------------------------------------------------------------
# opencode-broker systemd Service
# -----------------------------------------------------------------------------
# Create opencode-broker service for PAM authentication
# NOTE: Requires root privileges to write to /etc/systemd/system/
COPY packages/core/src/docker/files/opencode-broker.service /etc/systemd/system/opencode-broker.service

# Enable opencode-broker service
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode-broker.service /etc/systemd/system/multi-user.target.wants/opencode-broker.service

# -----------------------------------------------------------------------------
# opencode systemd Service (2026-01-22)
# -----------------------------------------------------------------------------
# Create opencode as a systemd service for Cockpit integration
# NOTE: Requires root privileges to write to /etc/systemd/system/
COPY packages/core/src/docker/files/opencode.service /etc/systemd/system/opencode.service

# Enable opencode service to start at boot (manual symlink since systemctl doesn't work during build)
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode.service /etc/systemd/system/multi-user.target.wants/opencode.service

# -----------------------------------------------------------------------------
# opencode Configuration
# -----------------------------------------------------------------------------
# Create opencode.jsonc config file with PAM authentication enabled
RUN mkdir -p /home/opencode/.config/opencode
COPY --chown=opencode:opencode packages/core/src/docker/files/opencode.jsonc /home/opencode/.config/opencode/opencode.jsonc
RUN chown -R opencode:opencode /home/opencode/.config/opencode \
    && chmod 644 /home/opencode/.config/opencode/opencode.jsonc

# Verify config file exists
RUN ls -la /home/opencode/.config/opencode/opencode.jsonc && cat /home/opencode/.config/opencode/opencode.jsonc

# -----------------------------------------------------------------------------
# Entrypoint Script (Hybrid Init Support)
# -----------------------------------------------------------------------------
# Supports both tini (default, works everywhere) and systemd (for Cockpit on Linux)
# Set USE_SYSTEMD=1 environment variable to use systemd init
# Note: Entrypoint runs as root to support both modes; tini mode drops to opencode user
COPY packages/core/src/docker/files/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Note: Don't set USER here - entrypoint needs root to use runuser
# The tini mode drops privileges to opencode user via runuser

# -----------------------------------------------------------------------------
# Health Check
# -----------------------------------------------------------------------------
# Check that opencode main page responds
# Works for both tini and systemd modes
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD ["sh", "-c", "OPENCODE_PORT=\"${OPENCODE_PORT:-${PORT:-3000}}\"; curl -f -H \"Accept: text/html\" \"http://localhost:${OPENCODE_PORT}/\" || exit 1"]

# -----------------------------------------------------------------------------
# opencode Artifacts
# -----------------------------------------------------------------------------
COPY --from=opencode-build /opt/opencode /opt/opencode
COPY --from=opencode-build /usr/local/bin/opencode-broker /usr/local/bin/opencode-broker

RUN chown -R opencode:opencode /opt/opencode \
    && chmod +x /opt/opencode/bin/opencode \
    && chmod 4755 /usr/local/bin/opencode-broker

# Verify broker binary exists and is executable
RUN ls -la /usr/local/bin/opencode-broker \
    && test -x /usr/local/bin/opencode-broker \
    && echo "Broker installed"

USER opencode
RUN /opt/opencode/bin/opencode --version

# -----------------------------------------------------------------------------
# GSD Plugin Installation
# -----------------------------------------------------------------------------
# Install the GSD (Get Shit Done) plugin for opencode
# Note: If this fails in container builds due to "~" path resolution, retry with
# OPENCODE_CONFIG_DIR=/home/opencode/.config/opencode set explicitly.
RUN mkdir -p /home/opencode/.npm \
    && npx --yes get-shit-done-cc@1.11.1 --opencode --global \
    && rm -rf /home/opencode/.npm/_cacache /home/opencode/.npm/_npx

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
