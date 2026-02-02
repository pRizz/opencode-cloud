# opencode-cloud-sandbox

Opinionated container image for AI-assisted coding with opencode.

Preferred usage and management is via the opencode-cloud CLI, which handles image pulls, volumes, ports, and upgrades:
https://github.com/pRizz/opencode-cloud

## What is included

- Ubuntu 24.04 (noble)
- Non-root user with passwordless sudo
- mise-managed runtimes (Node.js LTS, Python 3.12, Go 1.24)
- Rust toolchain via rustup
- Core CLI utilities (ripgrep, eza, jq, git, etc.)
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
docker run --rm -it -p 3000:3000 ghcr.io/prizz/opencode-cloud-sandbox:latest
```

The opencode web UI is available at `http://localhost:3000`.

## Install the opencode-cloud CLI

Cargo:

```
cargo install opencode-cloud
```

NPM:

```
npm install -g opencode-cloud
```

Then start the service (recommended):

```
occ start
```

## opencode build and serve flow

The Docker image builds opencode directly from the fork and runs the web server without nginx:

1. `cd packages/opencode`
2. `bun run build` to generate `packages/opencode/dist`
3. Run the server with `./bin/opencode web`

## Source

- Repository: https://github.com/pRizz/opencode-cloud
- Dockerfile: packages/core/src/docker/Dockerfile
