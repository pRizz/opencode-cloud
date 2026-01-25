# opencode-cloud-sandbox

Opinionated container image for AI-assisted coding with opencode.

## What is included

- Ubuntu 24.04 (noble)
- Non-root user with passwordless sudo
- mise-managed runtimes (Node.js LTS, Python 3.12, Go 1.24)
- Rust toolchain via rustup
- Core CLI utilities (ripgrep, eza, jq, git, etc.)
- Cockpit web console for administration
- opencode preinstalled with the GSD plugin

## Tags

- `latest`: Most recent published release
- `X.Y.Z`: Versioned releases (recommended for pinning)

## Usage

Pull the image:

```
docker pull ghcr.io/prizz/opencode-cloud-sandbox:latest
```

Run the container:

```
docker run --rm -it -p 3000:3000 -p 9090:9090 ghcr.io/prizz/opencode-cloud-sandbox:latest
```

The opencode web UI is available at `http://localhost:3000`. Cockpit runs on `http://localhost:9090`.

## Source

- Repository: https://github.com/pRizz/opencode-cloud
- Dockerfile: packages/core/src/docker/Dockerfile
