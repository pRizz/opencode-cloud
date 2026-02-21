# =============================================================================
# opencode-cloud Container Image
# =============================================================================
# IMPORTANT:
# Keep scripts/service/config assets in `packages/core/src/docker/files/`
# and COPY them into the image instead of embedding large inline shell blocks.
# When adding files, also update `packages/core/src/docker/image.rs`
# (`create_build_context`) so CLI-driven Docker builds include them.
#
# Build Hygiene Rules:
# - Prefer `RUN --mount=type=cache` for package caches (`apt`, `bun`, `cargo`,
#   `pip`, and `npm`) when BuildKit is available. New RUN steps that
#   download or compile dependencies should reuse existing cache mounts or add
#   new ones following the same patterns:
#     - APT:   --mount=type=cache,target=/var/lib/apt/lists
#              --mount=type=cache,target=/var/cache/apt
#     - Cargo: --mount=type=cache,id=cargo-registry-${TARGETARCH},target=...registry,uid=1000,gid=1000,mode=0755
#              (also cargo-git and cargo-target; scope per TARGETARCH for multi-platform)
#     - Bun:   --mount=type=cache,target=.../.bun/install/cache,uid=1000,gid=1000,mode=0755
#     - pip:   --mount=type=cache,target=.../.cache/pip,uid=1000,gid=1000,mode=0755
#   For non-root caches, always set uid/gid/mode and add a `chown -R` guard
#   at the start of the RUN body to fix subdirectory ownership from prior builds.
# - Remove temp workdirs in the same layer where they are created (for example
#   `/tmp/opencode-repo`) so transient artifacts do not persist in image layers.
# - Do not rely on cleanup in later layers; deleted files still exist in lower
#   layers and continue to bloat the final image history.
# - Keep builder-stage artifacts out of runtime by copying only final outputs.
# - When adding Docker build assets, update build-context inclusion logic in
#   `packages/core/src/docker/image.rs`.
# - When expanding local submodule packaging, keep dev-metadata excludes aligned
#   (`.planning`, `.git`, `node_modules`, `target`, etc.).
#
# A comprehensive development environment for AI-assisted coding with opencode.
#
# Features:
# - Ubuntu 26.04 LTS (resolute) base
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
# Last version audit: 2026-02-11
# -----------------------------------------------------------------------------

# -----------------------------------------------------------------------------
# Stage 1: Base
# -----------------------------------------------------------------------------
FROM ubuntu:26.04 AS base

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
LABEL org.opencontainers.image.base.name="ubuntu:26.04"

# Environment configuration
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# Runtime user identity (override at build time if host volume mapping needs it).
# These are preferred IDs, not strict requirements:
# - If OPENCODER_GID is already occupied, we create `opencoder` with the next free GID.
# - If OPENCODER_UID is already occupied, we create `opencoder` with the next free UID.
# This avoids collisions with base-image users (for example Ubuntu's default uid/gid 1000)
# while still allowing operators to request specific IDs when available.
ARG OPENCODER_UID=1000
ARG OPENCODER_GID=1000

# -----------------------------------------------------------------------------
# System Dependencies
# -----------------------------------------------------------------------------
# Install core system packages in logical groups for better caching

# Group 1: Core utilities and build tools (2026-02-11)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    # Init systems
    tini=0.19.* \
    dumb-init=1.2.* \
    # systemd for Cockpit support
    # UNPINNED: systemd - 26.04 base image ships libsystemd0 257 while repo has 259;
    # version pin fails during the implicit upgrade. Re-pin after 26.04 GA release.
    systemd \
    systemd-sysv \
    dbus=1.16.* \
    # Shell and terminal
    tmux=3.6a-* \
    # Editors
    vim=2:9.1.* \
    neovim=0.11.* \
    nano=8.7.* \
    # Build essentials
    build-essential=12.* \
    pkg-config=1.8.* \
    cmake=4.1.* \
    # Version control
    git=1:2.51.* \
    git-lfs=3.7.* \
    # Core utilities
    curl=8.18.* \
    wget=1.25.* \
    # UNPINNED: ca-certificates - security-critical root certs, needs auto-updates
    ca-certificates \
    # UNPINNED: gnupg - key management security, needs auto-updates
    gnupg \
    lsb-release=12.* \
    software-properties-common=0.119* \
    sudo=1.9.* \
    # UNPINNED: openssh-client - security-critical, needs auto-updates
    openssh-client \
    # Process/system tools
    htop=3.4.* \
    procps=2:4.0.* \
    less=668-* \
    file=1:5.46-* \
    tree=2.3.* \
    # JSON/YAML processing
    jq=1.8.* \
    # Network tools
    netcat-openbsd=1.234-* \
    iputils-ping=3:20250605-* \
    bind9-dnsutils=1:9.20.* \
    # Compression
    zip=3.0-* \
    unzip=6.0-* \
    xz-utils=5.8.* \
    # p7zip-full was removed in 26.04; replaced by 7zip (upstream 7-Zip)
    7zip=25.* \
    && rm -rf /var/lib/apt/lists/*

# Mask unnecessary systemd services for container environment
RUN systemctl mask \
    dev-hugepages.mount \
    sys-fs-fuse-connections.mount \
    systemd-update-utmp.service \
    systemd-tmpfiles-setup.service \
    systemd-remount-fs.service

# Group 2: Database clients (2026-02-11)
# Use BuildKit cache mount for APT package lists and cache
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    sqlite3=3.46.* \
    postgresql-client=18+* \
    default-mysql-client=1.1.* \
    && rm -rf /var/lib/apt/lists/*

# Group 3: Development libraries for compiling tools (2026-02-11)
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
    libncurses-dev=6.6+* \
    libpam0g-dev=1.7.* \
    liblzma-dev=5.8.* \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Create Non-Root User
# -----------------------------------------------------------------------------
# Create 'opencoder' user with passwordless sudo
RUN set -eux; \
    # Create group first. Prefer requested GID; fall back if already in use. \
    if ! getent group opencoder >/dev/null; then \
        if getent group "${OPENCODER_GID}" >/dev/null; then \
            groupadd opencoder; \
        else \
            groupadd --gid "${OPENCODER_GID}" opencoder; \
        fi; \
    fi; \
    # Create user next. Prefer requested UID; fall back if already in use. \
    if ! id -u opencoder >/dev/null 2>&1; then \
        if getent passwd "${OPENCODER_UID}" >/dev/null; then \
            useradd -m -s /bin/bash --gid opencoder -G sudo opencoder; \
        else \
            useradd -m -s /bin/bash --uid "${OPENCODER_UID}" --gid opencoder -G sudo opencoder; \
        fi; \
    fi; \
    # Always pin the primary group to `opencoder` in case the user pre-existed. \
    usermod --gid opencoder opencoder \
    && echo "opencoder ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/opencoder \
    && chmod 0440 /etc/sudoers.d/opencoder \
    && chmod 0750 /home/opencoder \
    # Snapshot built-in home users from the image so runtime auth logic can
    # ignore defaults (e.g. ubuntu) when deciding if onboarding should run.
    && install -d -m 0755 /etc/opencode-cloud \
    && awk -F: '$6 ~ /^\/home\// {print $1}' /etc/passwd | sort -u > /etc/opencode-cloud/builtin-home-users.txt \
    && chmod 0644 /etc/opencode-cloud/builtin-home-users.txt

# Switch to opencode user for remaining setup
USER opencoder
WORKDIR /home/opencoder

# Set up directories
RUN mkdir -p \
    /home/opencoder/.config \
    /home/opencoder/.local/bin \
    /home/opencoder/.local/share \
    /home/opencoder/.local/state \
    /home/opencoder/.cache \
    /home/opencoder/workspace

# Add local bin to PATH
ENV PATH="/home/opencoder/.local/bin:${PATH}"

# -----------------------------------------------------------------------------
# Shell Setup: Bash + Starship
# -----------------------------------------------------------------------------
# Starship prompt - self-managing installer, trusted to handle versions
# Disabled temporarily to reduce Docker build time.
# RUN curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /home/opencoder/.local/bin

# Configure bash with starship
# Disabled temporarily to reduce Docker build time.
# RUN echo 'eval "$(starship init bash)"' >> /home/opencoder/.bashrc \
#     && echo 'export PATH="/home/opencoder/.local/bin:$PATH"' >> /home/opencoder/.bashrc

# -----------------------------------------------------------------------------
# mise: Universal Version Manager
# -----------------------------------------------------------------------------
# mise - self-managing installer, trusted to handle versions
RUN curl https://mise.run | sh \
    && echo 'eval "$(/home/opencoder/.local/bin/mise activate bash)"' >> /home/opencoder/.bashrc

# Install language runtimes via mise (2026-02-11)
# - node@25: pinned to major version
# - python@3.13: pinned to major version (3.13 has precompiled binaries available)
# - go@1.26: pinned to minor version
#
# MISE_PYTHON_COMPILE=0: Disable Python compilation from source.
# If no precompiled binary exists, mise will fail fast instead of hanging
# during a 30+ minute compilation that may timeout in CI.
#
# Timeout: 10 minutes max for all runtime installations as safety net.
RUN timeout 600 sh -c '\
    export MISE_PYTHON_COMPILE=0 && \
    /home/opencoder/.local/bin/mise install node@25 && \
    /home/opencoder/.local/bin/mise install python@3.13 && \
    /home/opencoder/.local/bin/mise install go@1.26 && \
    /home/opencoder/.local/bin/mise use --global node@25 && \
    /home/opencoder/.local/bin/mise use --global python@3.13 && \
    /home/opencoder/.local/bin/mise use --global go@1.26'

# Set up mise shims in PATH for non-interactive shells
ENV PATH="/home/opencoder/.local/share/mise/shims:${PATH}"

# -----------------------------------------------------------------------------
# Rust Installation
# -----------------------------------------------------------------------------
# rustup - self-managing installer, trusted to handle versions
# Uses stable toolchain with minimal profile (skip docs and extras)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
    && . /home/opencoder/.cargo/env \
    && rustup default stable \
    && rustup component add rust-analyzer rustfmt clippy

ENV PATH="/home/opencoder/.cargo/bin:${PATH}"

# -----------------------------------------------------------------------------
# Package Managers
# -----------------------------------------------------------------------------
# Switch to bash for mise activation (mise outputs bash-specific syntax)
SHELL ["/bin/bash", "-c"]

# bun - self-managing installer, pinned to version (2026-02-03)
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.3.9" \
    && rm -rf /home/opencoder/.bun/install/cache /home/opencoder/.bun/cache /home/opencoder/.cache/bun

ENV PATH="/home/opencoder/.bun/bin:${PATH}"

# uv - pinned installer (fast Python package manager) (2026-02-03)
RUN curl -LsSf https://github.com/astral-sh/uv/releases/download/0.9.21/uv-installer.sh | sh

# Install pipx for isolated Python application installs
# Use BuildKit cache mount for pip cache
RUN --mount=type=cache,target=/home/opencoder/.cache/pip,uid=1000,gid=1000,mode=0755 \
    mkdir -p /home/opencoder/.cache/pip \
    && eval "$(/home/opencoder/.local/bin/mise activate bash)" \
    && pip install --user pipx \
    && pipx ensurepath

# Install global TypeScript compiler
RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
    && bun add -g typescript@5.9.2

# -----------------------------------------------------------------------------
# Modern CLI Tools - pre-built release binaries (2026-01-22)
# -----------------------------------------------------------------------------
# Download pre-built binaries instead of compiling from source via cargo install.
# This avoids 3-5 min cargo compilation for tools with pinned versions, replacing
# it with ~5 second downloads. Also eliminates dependency on cargo cache mounts
# which are empty on ephemeral CI runners (GitHub Actions).
ARG TARGETARCH

# ripgrep 15.1.0 - fast regex search
RUN set -eux; \
    case "${TARGETARCH}" in \
      amd64) RG_ARCH="x86_64-unknown-linux-musl" ;; \
      arm64) RG_ARCH="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/BurntSushi/ripgrep/releases/download/15.1.0/ripgrep-15.1.0-${RG_ARCH}.tar.gz" \
      | tar -xz --strip-components=1 -C /home/opencoder/.local/bin/ "ripgrep-15.1.0-${RG_ARCH}/rg"; \
    rg --version

# eza 0.23.4 - modern ls replacement
RUN set -eux; \
    case "${TARGETARCH}" in \
      amd64) EZA_ARCH="x86_64-unknown-linux-gnu" ;; \
      arm64) EZA_ARCH="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/eza-community/eza/releases/download/v0.23.4/eza_${EZA_ARCH}.tar.gz" \
      | tar -xz --strip-components=0 -C /home/opencoder/.local/bin/; \
    eza --version

# lazygit v0.58.1 (2026-01-12) - terminal UI for git
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && go install github.com/jesseduffield/lazygit@v0.59.0

# -----------------------------------------------------------------------------
# Additional Development Tools
# -----------------------------------------------------------------------------
# fzf v0.67.0 (2025-11-16) - fuzzy finder
# Disabled temporarily to reduce Docker build time.
# RUN git clone --branch v0.67.0 --depth 1 https://github.com/junegunn/fzf.git /home/opencoder/.fzf \
#     && /home/opencoder/.fzf/install --all --no-bash --no-fish

# yq v4.50.1 (2025-12-14) - YAML processor
# Disabled temporarily to reduce Docker build time.
# RUN curl -sL https://github.com/mikefarah/yq/releases/download/v4.52.2/yq_linux_$(dpkg --print-architecture) -o /home/opencoder/.local/bin/yq \
#     && chmod +x /home/opencoder/.local/bin/yq

# Install direnv (2026-01-22)
# Disabled temporarily to reduce Docker build time.
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends direnv=2.32.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencoder
# RUN echo 'eval "$(direnv hook bash)"' >> /home/opencoder/.bashrc

# Install HTTPie
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && pipx install httpie

# Install shellcheck (2026-01-22)
# Disabled temporarily to reduce Docker build time.
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends shellcheck=0.9.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencoder
# shfmt v3.12.0 (2025-07-06) - shell formatter
# Disabled temporarily to reduce Docker build time.
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && go install mvdan.cc/sh/v3/cmd/shfmt@v3.12.0

# Install btop system monitor (2026-02-03)
# Use BuildKit cache mount for APT package lists and cache
USER root
RUN --mount=type=cache,target=/var/lib/apt/lists \
    --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends btop=1.4.* \
    && rm -rf /var/lib/apt/lists/*
USER opencoder

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
USER opencoder

# -----------------------------------------------------------------------------
# Cockpit Web Console (2026-01-22) - temporarily disabled
# -----------------------------------------------------------------------------
# Cockpit provides web-based administration for the container
# Ubuntu resolute has cockpit in main repos
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
# USER opencoder

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
# RUN curl -sL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash -s -- -b /home/opencoder/.local/bin v0.2.84
#
# # -----------------------------------------------------------------------------
# # Rust Tooling - pinned versions (2026-01-22)
# # -----------------------------------------------------------------------------
# # cargo-nextest 0.9.122 - fast test runner
# # cargo-audit 0.22.0 - security audit
# # cargo-deny 0.19.0 - dependency linter
# RUN . /home/opencoder/.cargo/env \
#     && cargo install --locked cargo-nextest@0.9.126 cargo-audit@0.22.1 cargo-deny@0.19.0
#
# # Install mold fast linker (2026-01-22)
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends mold=2.30.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencoder
#
# # -----------------------------------------------------------------------------
# # Code Quality Tools
# # -----------------------------------------------------------------------------
# # JavaScript/TypeScript tools
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && bun add -g \
#     prettier \
#     eslint \
#     @biomejs/biome
#
# # Python tools
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && pipx install black \
#     && pipx install ruff
#
# # Test runners (commonly needed)
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && bun add -g jest vitest
#
# # Python pytest via pipx
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
#     && pipx install pytest
#
# # -----------------------------------------------------------------------------
# # Protocol Buffers / gRPC (2026-01-22)
# # -----------------------------------------------------------------------------
# USER root
# RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler=3.21.* \
#     && rm -rf /var/lib/apt/lists/*
# USER opencoder
#
# # grpcurl v1.9.3 (2025-03-11) - gRPC debugging tool
# RUN eval "$(/home/opencoder/.local/bin/mise activate bash)" \
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
COPY --chown=opencoder:opencoder packages/core/src/docker/files/starship.toml /home/opencoder/.config/starship.toml

# Shell aliases
COPY --chown=opencoder:opencoder packages/core/src/docker/files/bashrc.extra /home/opencoder/.bashrc.extra
RUN cat /home/opencoder/.bashrc.extra >> /home/opencoder/.bashrc \
    && rm /home/opencoder/.bashrc.extra

# =============================================================================
# Stage 2a: opencode-source — Obtain source code
# =============================================================================
# Separated into its own stage so downstream stages (JS build, broker build)
# can COPY only what they need, enabling BuildKit to run them in parallel.
FROM base AS opencode-source

USER opencoder
ARG OPENCODE_SOURCE=remote
ARG OPENCODE_COMMIT
ARG OPENCODE_LOCAL_REF

# CLI builds use a custom build-context generator that always adds
# packages/opencode. Repo-root Docker builds rely on checkout + .dockerignore to
# keep this path as a tiny placeholder in remote mode.
# In local dev mode it contains the full submodule.
COPY --chown=opencoder:opencoder packages/opencode /tmp/opencode-local

# Clone the fork or copy local source.
# NOTE: OPENCODE_COMMIT is not tied to releases/tags; it tracks the latest stable
# commit on the dev branch of https://github.com/pRizz/opencode.
# Update it by running: ./scripts/update-opencode-commit.sh
RUN set -eux; \
    OPENCODE_COMMIT_OVERRIDE="${OPENCODE_COMMIT:-}"; \
    OPENCODE_COMMIT="65d53d330955a759cca6bac50039a2a923da7fb9"; \
    if [ -n "${OPENCODE_COMMIT_OVERRIDE}" ]; then OPENCODE_COMMIT="${OPENCODE_COMMIT_OVERRIDE}"; fi; \
    rm -rf /tmp/opencode-repo; \
    if [ "${OPENCODE_SOURCE}" = "local" ]; then \
        if [ ! -f /tmp/opencode-local/package.json ]; then \
            echo "Local opencode source requested but packages/opencode was not included in build context."; \
            exit 1; \
        fi; \
        mkdir -p /tmp/opencode-repo; \
        cp -R /tmp/opencode-local/. /tmp/opencode-repo; \
    else \
        git clone --depth 1 https://github.com/pRizz/opencode.git /tmp/opencode-repo; \
        cd /tmp/opencode-repo; \
        git fetch --depth 1 origin "${OPENCODE_COMMIT}"; \
        git checkout "${OPENCODE_COMMIT}"; \
    fi; \
    rm -rf /tmp/opencode-local

# =============================================================================
# Stage 2b: opencode-js-build — bun install + UI build (runs in parallel with broker)
# =============================================================================
FROM base AS opencode-js-build

USER opencoder
ARG OPENCODE_SOURCE=remote
ARG TARGETARCH

# Bind-mount the source from the opencode-source stage, then copy it into
# this layer's writable filesystem in the same RUN as bun install. This avoids
# issues where Docker COPY --from= between stages can cause bun install to
# fail with ENOENT during platform-specific package linking.
# Reliability note for Bun dependency install:
# - We use a dedicated Bun install cache mount so BuildKit can reuse downloaded
#   packages across builds without polluting image layers.
# - In CI/container builds, Bun's cached install artifacts can occasionally
#   become inconsistent (for example after interrupted network/download steps),
#   which causes `bun install --frozen-lockfile` to fail nondeterministically.
#   Common symptoms: integrity check failures for platform-specific tarballs.
# - The retry loop aggressively clears ALL bun caches AND node_modules between
#   attempts to ensure each retry starts from a truly clean state.
# - On successful installs, keep the BuildKit cache mount populated so future
#   builds can reuse downloaded tarballs.
RUN --mount=type=bind,from=opencode-source,source=/tmp/opencode-repo,target=/tmp/opencode-source-ro \
    # Keep cache warm, isolate by architecture, and serialize writers to reduce
    # cross-arch contamination and concurrent cache corruption.
    --mount=type=cache,id=bun-install-${TARGETARCH},target=/home/opencoder/.bun/install/cache,uid=1000,gid=1000,mode=0755,sharing=locked \
    cp -R /tmp/opencode-source-ro /tmp/opencode-repo \
    && sudo mkdir -p /home/opencoder/.bun/install/cache \
    && sudo chown -R opencoder:opencoder /home/opencoder/.bun/install/cache \
    && export BUN_INSTALL_CACHE_DIR=/home/opencoder/.bun/install/cache \
    && cd /tmp/opencode-repo \
    && bun_install_ok=0; \
      for attempt in 1 2 3; do \
        if bun install --frozen-lockfile; then \
          bun_install_ok=1; \
          break; \
        fi; \
        echo "bun install failed (attempt ${attempt}/3); purging all caches and retrying..." >&2; \
        sudo find /home/opencoder/.bun/install/cache -mindepth 1 -maxdepth 1 -exec rm -rf {} + || true; \
        sudo rm -rf /home/opencoder/.bun/cache /home/opencoder/.cache/bun || true; \
        rm -rf node_modules packages/*/node_modules packages/*/*/node_modules || true; \
      done; \
      if [ "${bun_install_ok}" -ne 1 ]; then \
        echo "bun install failed after 3 attempts." >&2; \
        exit 1; \
      fi \
    && cd packages/opencode \
    && if [ "${OPENCODE_SOURCE}" = "local" ]; then \
        OPENCODE_CHANNEL=local bun run build-single-ui; \
      else \
        bun run build-single-ui; \
      fi

# =============================================================================
# Stage 2c: broker-planner — Prepare cargo-chef dependency recipe
# =============================================================================
# cargo-chef separates dependency compilation from application compilation.
# The recipe captures only the dependency graph (Cargo.toml/Cargo.lock), so the
# broker-deps layer below only invalidates when dependencies change, not when
# broker source code changes.
FROM base AS broker-planner

USER opencoder
ARG TARGETARCH

# Install cargo-chef from pre-built binary (avoids cargo install compilation time)
RUN set -eux; \
    case "${TARGETARCH}" in \
      amd64) CHEF_ARCH="x86_64-unknown-linux-musl" ;; \
      arm64) CHEF_ARCH="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/latest/download/cargo-chef-${CHEF_ARCH}.tar.gz" \
      | tar -xz -C /home/opencoder/.local/bin/; \
    cargo-chef --version

# Intentionally copy only Cargo manifests here so cargo-chef dependency planning
# is invalidated by dependency graph changes, not by broker source edits.
RUN mkdir -p /tmp/opencode-broker
COPY --from=opencode-source /tmp/opencode-repo/packages/opencode-broker/Cargo.toml /tmp/opencode-broker/Cargo.toml
COPY --from=opencode-source /tmp/opencode-repo/packages/opencode-broker/Cargo.lock /tmp/opencode-broker/Cargo.lock
WORKDIR /tmp/opencode-broker
RUN . /home/opencoder/.cargo/env \
    && cargo chef prepare --recipe-path /tmp/recipe.json

# =============================================================================
# Stage 2d: broker-deps — Build broker dependencies only (cached when deps unchanged)
# =============================================================================
FROM base AS broker-deps

USER opencoder
ARG TARGETARCH

# Install cargo-chef (needed for cargo chef cook)
RUN set -eux; \
    case "${TARGETARCH}" in \
      amd64) CHEF_ARCH="x86_64-unknown-linux-musl" ;; \
      arm64) CHEF_ARCH="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/latest/download/cargo-chef-${CHEF_ARCH}.tar.gz" \
      | tar -xz -C /home/opencoder/.local/bin/

# Copy only the recipe (dependency graph). This layer is cached as long as
# Cargo.toml/Cargo.lock are unchanged — broker source code changes don't
# invalidate it.
COPY --from=broker-planner /tmp/recipe.json /tmp/recipe.json

# Build all dependencies. Cargo cache mounts provide incremental benefit when
# this layer does invalidate (new deps added).
# All cargo caches scoped per TARGETARCH to prevent multi-platform corruption.
WORKDIR /tmp/broker-build
RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/home/opencoder/.cargo/registry,uid=1000,gid=1000,mode=0755 \
    --mount=type=cache,id=cargo-git-${TARGETARCH},target=/home/opencoder/.cargo/git,uid=1000,gid=1000,mode=0755 \
    sudo chown -R opencoder:opencoder /home/opencoder/.cargo/registry /home/opencoder/.cargo/git \
    && . /home/opencoder/.cargo/env \
    && cargo chef cook --release --recipe-path /tmp/recipe.json

# =============================================================================
# Stage 2e: broker-build — Compile broker application (deps already built)
# =============================================================================
# This stage runs in parallel with opencode-js-build. Only the broker's own
# crate is recompiled here; all dependencies come from the broker-deps layer.
FROM broker-deps AS broker-build

USER opencoder
ARG TARGETARCH

COPY --from=opencode-source /tmp/opencode-repo/packages/opencode-broker /tmp/broker-build
RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/home/opencoder/.cargo/registry,uid=1000,gid=1000,mode=0755 \
    --mount=type=cache,id=cargo-git-${TARGETARCH},target=/home/opencoder/.cargo/git,uid=1000,gid=1000,mode=0755 \
    sudo chown -R opencoder:opencoder /home/opencoder/.cargo/registry /home/opencoder/.cargo/git \
    && . /home/opencoder/.cargo/env \
    && cargo build --release

# =============================================================================
# Stage 2f: opencode-build — Assemble final artifacts
# =============================================================================
FROM base AS opencode-build

USER opencoder
ARG OPENCODE_SOURCE=remote
ARG OPENCODE_COMMIT
ARG OPENCODE_LOCAL_REF

# Copy built JS/UI dist from the JS build stage
COPY --from=opencode-js-build /tmp/opencode-repo/packages/opencode/dist /tmp/opencode-dist

# Copy built broker binary from the broker build stage
COPY --from=broker-build /tmp/broker-build/target/release/opencode-broker /tmp/opencode-broker-bin

# Assemble artifacts into final locations
RUN set -eux; \
    OPENCODE_COMMIT_OVERRIDE="${OPENCODE_COMMIT:-}"; \
    OPENCODE_LOCAL_REF="${OPENCODE_LOCAL_REF:-local-unknown}"; \
    OPENCODE_COMMIT="e3cbc7b4611688309e3e7b0004987679e94d3392"; \
    if [ -n "${OPENCODE_COMMIT_OVERRIDE}" ]; then OPENCODE_COMMIT="${OPENCODE_COMMIT_OVERRIDE}"; fi; \
    sudo mkdir -p /opt/opencode/bin /opt/opencode/ui; \
    if [ "${OPENCODE_SOURCE}" = "local" ]; then \
        echo "${OPENCODE_LOCAL_REF}" | sudo tee /opt/opencode/COMMIT >/dev/null; \
    else \
        echo "${OPENCODE_COMMIT}" | sudo tee /opt/opencode/COMMIT >/dev/null; \
    fi; \
    sudo chown opencoder:opencoder /opt/opencode/COMMIT; \
    sudo cp /tmp/opencode-dist/opencode-*/bin/opencode /opt/opencode/bin/opencode; \
    sudo cp -R /tmp/opencode-dist/opencode-*/ui/. /opt/opencode/ui/; \
    sudo chown -R opencoder:opencoder /opt/opencode; \
    sudo chmod +x /opt/opencode/bin/opencode; \
    sudo mkdir -p /usr/local/bin; \
    sudo cp /tmp/opencode-broker-bin /usr/local/bin/opencode-broker; \
    sudo chmod 4755 /usr/local/bin/opencode-broker; \
    sudo rm -rf /tmp/opencode-dist /tmp/opencode-broker-bin

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
COPY --chown=root:root --chmod=0644 packages/core/src/docker/files/pam/opencode /etc/pam.d/opencode

# -----------------------------------------------------------------------------
# opencode-broker systemd Service
# -----------------------------------------------------------------------------
# Create opencode-broker service for PAM authentication
# NOTE: Requires root privileges to write to /etc/systemd/system/
COPY --chown=root:root --chmod=0644 packages/core/src/docker/files/opencode-broker.service /etc/systemd/system/opencode-broker.service

# Enable opencode-broker service
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode-broker.service /etc/systemd/system/multi-user.target.wants/opencode-broker.service

# -----------------------------------------------------------------------------
# opencode systemd Service (2026-01-22)
# -----------------------------------------------------------------------------
# Create opencode as a systemd service for Cockpit integration
# NOTE: Requires root privileges to write to /etc/systemd/system/
COPY --chown=root:root --chmod=0644 packages/core/src/docker/files/opencode.service /etc/systemd/system/opencode.service

# Enable opencode service to start at boot (manual symlink since systemctl doesn't work during build)
RUN mkdir -p /etc/systemd/system/multi-user.target.wants \
    && ln -sf /etc/systemd/system/opencode.service /etc/systemd/system/multi-user.target.wants/opencode.service

# -----------------------------------------------------------------------------
# opencode Configuration
# -----------------------------------------------------------------------------
# Create opencode.jsonc config file with PAM authentication enabled
RUN install -d -o opencoder -g opencoder -m 0750 /home/opencoder/.config/opencode
COPY --chown=opencoder:opencoder --chmod=0640 packages/core/src/docker/files/opencode.jsonc /home/opencoder/.config/opencode/opencode.jsonc

# Default opencode config template (survives volume mounts over ~/.config/opencode)
RUN install -d -m 0755 /etc/opencode-cloud
COPY --chown=root:root --chmod=0644 packages/core/src/docker/files/opencode.jsonc /etc/opencode-cloud/opencode.jsonc.default

# -----------------------------------------------------------------------------
# Entrypoint Script (Hybrid Init Support)
# -----------------------------------------------------------------------------
# Supports both tini (default, works everywhere) and systemd (for Cockpit on Linux)
# Set USE_SYSTEMD=1 environment variable to use systemd init
# Note: Entrypoint runs as root to support both modes; tini mode drops to opencode user
COPY --chown=root:root --chmod=0755 packages/core/src/docker/files/entrypoint.sh /usr/local/bin/entrypoint.sh

# Bootstrap helper for first-user onboarding (invoked by entrypoint and auth route via sudo)
COPY --chown=root:root --chmod=0700 packages/core/src/docker/files/opencode-cloud-bootstrap.sh /usr/local/bin/opencode-cloud-bootstrap

# Note: Don't set USER here - entrypoint needs root to use runuser
# The tini mode drops privileges to opencode user via runuser

# Healthcheck script asset
COPY --chown=root:root --chmod=0755 packages/core/src/docker/files/healthcheck.sh /usr/local/bin/healthcheck.sh

# -----------------------------------------------------------------------------
# Health Check
# -----------------------------------------------------------------------------
# Check broker readiness (process + socket) and opencode web response
# Works for both tini and systemd modes
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD ["/usr/local/bin/healthcheck.sh"]

# -----------------------------------------------------------------------------
# opencode Artifacts
# -----------------------------------------------------------------------------
COPY --from=opencode-build /opt/opencode /opt/opencode
COPY --from=opencode-build /usr/local/bin/opencode-broker /usr/local/bin/opencode-broker

RUN chown -R root:root /opt/opencode \
    && chmod -R go-w /opt/opencode \
    && chmod +x /opt/opencode/bin/opencode \
    && chown root:root /usr/local/bin/opencode-broker \
    && chmod 4755 /usr/local/bin/opencode-broker

# Verify broker binary exists and is executable
RUN ls -la /usr/local/bin/opencode-broker \
    && test -x /usr/local/bin/opencode-broker \
    && echo "Broker installed"

USER opencoder
RUN /opt/opencode/bin/opencode --version

# -----------------------------------------------------------------------------
# GSD Plugin Installation
# -----------------------------------------------------------------------------
# Install the GSD (Get Shit Done) plugin for opencode
# Note: If this fails in container builds due to "~" path resolution, retry with
# OPENCODE_CONFIG_DIR=/home/opencoder/.config/opencode set explicitly.
RUN mkdir -p /home/opencoder/.npm \
    && npx --yes get-shit-done-cc@1.11.1 --opencode --global \
    && rm -rf /home/opencoder/.npm/_cacache /home/opencoder/.npm/_npx

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
WORKDIR /home/opencoder/workspace

# Declare volumes for critical persistent data paths.
# When the image is run directly (e.g., Railway, docker run) without the occ CLI,
# Docker creates anonymous volumes for these paths, providing persistence across
# container restarts (but NOT across container removal/recreation).
# When managed by occ, explicit named volume mounts override these directives.
VOLUME ["/home/opencoder/.local/share/opencode"]
VOLUME ["/home/opencoder/.local/state/opencode"]
VOLUME ["/home/opencoder/.cache/opencode"]
VOLUME ["/home/opencoder/workspace"]
VOLUME ["/home/opencoder/.config/opencode"]
VOLUME ["/var/lib/opencode-users"]
VOLUME ["/home/opencoder/.ssh"]

# Expose opencode web (3000) and Cockpit (9090)
EXPOSE 3000 9090

# Hybrid init: entrypoint script chooses tini or systemd based on USE_SYSTEMD env
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
