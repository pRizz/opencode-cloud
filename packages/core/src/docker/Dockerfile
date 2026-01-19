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
# - Container/K8s tools (kubectl, helm, k9s, dive)
# - Security tools (trivy, gitleaks, hadolint, age, sops)
# - GSD opencode plugin for task management
#
# Usage:
#   docker build -t opencode-cloud .
#   docker run -it opencode-cloud
#
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
# Install build dependencies and tools that require compilation
FROM ubuntu:24.04 AS builder

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install build essentials
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
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

# Group 1: Core utilities and build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Signal handling
    tini \
    dumb-init \
    # Shell and terminal
    zsh \
    tmux \
    # Editors
    vim \
    neovim \
    nano \
    # Build essentials
    build-essential \
    pkg-config \
    cmake \
    # Version control
    git \
    git-lfs \
    # Core utilities
    curl \
    wget \
    ca-certificates \
    gnupg \
    lsb-release \
    software-properties-common \
    sudo \
    openssh-client \
    # Process/system tools
    htop \
    procps \
    less \
    file \
    tree \
    # JSON/YAML processing
    jq \
    # Network tools
    netcat-openbsd \
    iputils-ping \
    dnsutils \
    # Compression
    zip \
    unzip \
    xz-utils \
    p7zip-full \
    && rm -rf /var/lib/apt/lists/*

# Group 2: Database clients
RUN apt-get update && apt-get install -y --no-install-recommends \
    sqlite3 \
    postgresql-client \
    default-mysql-client \
    && rm -rf /var/lib/apt/lists/*

# Group 3: Development libraries (for compiling tools)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    libffi-dev \
    zlib1g-dev \
    libbz2-dev \
    libreadline-dev \
    libsqlite3-dev \
    libncurses-dev \
    liblzma-dev \
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
# Install Oh My Zsh
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

# Install Starship prompt
RUN curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /home/opencode/.local/bin

# Configure zsh with starship
RUN echo 'eval "$(starship init zsh)"' >> /home/opencode/.zshrc \
    && echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

# -----------------------------------------------------------------------------
# mise: Universal Version Manager
# -----------------------------------------------------------------------------
# Install mise for managing Node.js, Python, Rust, Go
RUN curl https://mise.run | sh \
    && echo 'eval "$(/home/opencode/.local/bin/mise activate zsh)"' >> /home/opencode/.zshrc

# Install language runtimes via mise
# Using specific LTS/stable versions for reproducibility
RUN /home/opencode/.local/bin/mise install node@lts \
    && /home/opencode/.local/bin/mise install python@3.12 \
    && /home/opencode/.local/bin/mise install go@latest \
    && /home/opencode/.local/bin/mise use --global node@lts \
    && /home/opencode/.local/bin/mise use --global python@3.12 \
    && /home/opencode/.local/bin/mise use --global go@latest

# Set up mise shims in PATH for non-interactive shells
ENV PATH="/home/opencode/.local/share/mise/shims:${PATH}"

# -----------------------------------------------------------------------------
# Rust Installation
# -----------------------------------------------------------------------------
# Install Rust via rustup (mise rust support is experimental)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && . /home/opencode/.cargo/env \
    && rustup component add rust-analyzer rustfmt clippy

ENV PATH="/home/opencode/.cargo/bin:${PATH}"

# -----------------------------------------------------------------------------
# Package Managers
# -----------------------------------------------------------------------------
# Switch to bash for mise activation (mise outputs bash-specific syntax)
SHELL ["/bin/bash", "-c"]

# Install pnpm (corepack is included with Node.js)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && corepack enable \
    && corepack prepare pnpm@latest --activate

# Set up pnpm global bin directory
ENV PNPM_HOME="/home/opencode/.local/share/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"
RUN mkdir -p "${PNPM_HOME}"

# Install uv (fast Python package manager)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install global TypeScript compiler
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pnpm add -g typescript

# -----------------------------------------------------------------------------
# Modern CLI Tools (Rust-based)
# -----------------------------------------------------------------------------
# Install via cargo for latest versions
RUN . /home/opencode/.cargo/env \
    && cargo install --locked \
    ripgrep \
    eza

# Install lazygit (Go-based)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install github.com/jesseduffield/lazygit@latest

# -----------------------------------------------------------------------------
# Additional Development Tools
# -----------------------------------------------------------------------------
# Install fzf
RUN git clone --depth 1 https://github.com/junegunn/fzf.git /home/opencode/.fzf \
    && /home/opencode/.fzf/install --all --no-bash --no-fish

# Install yq (YAML processor)
RUN curl -sL https://github.com/mikefarah/yq/releases/latest/download/yq_linux_$(dpkg --print-architecture) -o /home/opencode/.local/bin/yq \
    && chmod +x /home/opencode/.local/bin/yq

# Install direnv
USER root
RUN apt-get update && apt-get install -y --no-install-recommends direnv \
    && rm -rf /var/lib/apt/lists/*
USER opencode
RUN echo 'eval "$(direnv hook zsh)"' >> /home/opencode/.zshrc

# Install HTTPie
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && pipx install httpie

# Install shellcheck and shfmt
USER root
RUN apt-get update && apt-get install -y --no-install-recommends shellcheck \
    && rm -rf /var/lib/apt/lists/*
USER opencode
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install mvdan.cc/sh/v3/cmd/shfmt@latest

# Install btop (system monitor)
USER root
RUN apt-get update && apt-get install -y --no-install-recommends btop \
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
# Container/Kubernetes Tools
# -----------------------------------------------------------------------------
# Install kubectl
USER root
RUN curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.31/deb/Release.key | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg \
    && echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.31/deb/ /' > /etc/apt/sources.list.d/kubernetes.list \
    && apt-get update && apt-get install -y --no-install-recommends kubectl \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# Install Helm
RUN curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash -s -- --no-sudo --install-dir /home/opencode/.local/bin

# Install k9s
RUN curl -sL https://github.com/derailed/k9s/releases/latest/download/k9s_Linux_$(dpkg --print-architecture).tar.gz | tar xz -C /home/opencode/.local/bin k9s

# Install dive (Docker image explorer)
RUN curl -sL https://github.com/wagoodman/dive/releases/latest/download/dive_$(curl -s https://api.github.com/repos/wagoodman/dive/releases/latest | grep tag_name | cut -d '"' -f 4 | sed 's/v//')_linux_$(dpkg --print-architecture).tar.gz | tar xz -C /home/opencode/.local/bin dive

# -----------------------------------------------------------------------------
# Security Tools
# -----------------------------------------------------------------------------
# Install trivy (security scanner)
USER root
RUN curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
USER opencode

# Install gitleaks (secret scanner)
RUN curl -sL https://github.com/gitleaks/gitleaks/releases/latest/download/gitleaks_$(curl -s https://api.github.com/repos/gitleaks/gitleaks/releases/latest | grep tag_name | cut -d '"' -f 4 | sed 's/v//')_linux_$([ "$(dpkg --print-architecture)" = "amd64" ] && echo "x64" || echo "arm64").tar.gz | tar xz -C /home/opencode/.local/bin gitleaks

# Install hadolint (Dockerfile linter)
RUN curl -sL https://github.com/hadolint/hadolint/releases/latest/download/hadolint-Linux-$([ "$(dpkg --print-architecture)" = "amd64" ] && echo "x86_64" || echo "arm64") -o /home/opencode/.local/bin/hadolint \
    && chmod +x /home/opencode/.local/bin/hadolint

# Install age (encryption)
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install filippo.io/age/cmd/...@latest

# Install sops (secrets operations)
RUN curl -sL https://github.com/getsops/sops/releases/latest/download/sops-v$(curl -s https://api.github.com/repos/getsops/sops/releases/latest | grep tag_name | cut -d '"' -f 4 | sed 's/v//')-linux.$(dpkg --print-architecture) -o /home/opencode/.local/bin/sops \
    && chmod +x /home/opencode/.local/bin/sops

# Install mkcert (local TLS)
RUN curl -sL https://github.com/FiloSottile/mkcert/releases/latest/download/mkcert-v$(curl -s https://api.github.com/repos/FiloSottile/mkcert/releases/latest | grep tag_name | cut -d '"' -f 4 | sed 's/v//')-linux-$(dpkg --print-architecture) -o /home/opencode/.local/bin/mkcert \
    && chmod +x /home/opencode/.local/bin/mkcert

# -----------------------------------------------------------------------------
# CI/CD Tools
# -----------------------------------------------------------------------------
# Install act (run GitHub Actions locally)
RUN curl -sL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash -s -- -b /home/opencode/.local/bin

# -----------------------------------------------------------------------------
# Rust Tooling
# -----------------------------------------------------------------------------
RUN . /home/opencode/.cargo/env \
    && cargo install --locked \
    cargo-nextest \
    cargo-audit \
    cargo-deny \
    sccache \
    cross

# Install mold (fast linker) via apt for easier setup
USER root
RUN apt-get update && apt-get install -y --no-install-recommends mold \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# Configure sccache
ENV RUSTC_WRAPPER="/home/opencode/.cargo/bin/sccache"

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
# Protocol Buffers / gRPC
# -----------------------------------------------------------------------------
USER root
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
USER opencode

# Install grpcurl
RUN eval "$(/home/opencode/.local/bin/mise activate bash)" \
    && go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# -----------------------------------------------------------------------------
# opencode Installation
# -----------------------------------------------------------------------------
# Install opencode using official install script
RUN curl -fsSL https://opencode.ai/install | bash

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

# Kubernetes aliases
alias k="kubectl"
alias kgp="kubectl get pods"
alias kgs="kubectl get services"
ALIASES

# Set up pipx path
RUN echo 'export PATH="/home/opencode/.local/bin:$PATH"' >> /home/opencode/.zshrc

# -----------------------------------------------------------------------------
# Health Check
# -----------------------------------------------------------------------------
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# -----------------------------------------------------------------------------
# Final Configuration
# -----------------------------------------------------------------------------
WORKDIR /home/opencode/workspace

# Expose default opencode web port
EXPOSE 8080

# Use tini as init system for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command: start opencode web interface
CMD ["opencode", "web"]
