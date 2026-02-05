# opencode-cloud-sandbox

Opinionated container image for AI-assisted coding with opencode.

Preferred usage and management is via the opencode-cloud CLI, which handles image pulls, volumes, ports, and upgrades:
https://github.com/pRizz/opencode-cloud (mirror: https://gitea.com/pRizz/opencode-cloud)

## What is included

- Ubuntu 25.10 (questing)
- Non-root user with passwordless sudo
- mise-managed runtimes (Node.js 25, Python 3.14, Go 1.25)
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

## App Platform

- Set `http_port` to `3000` or provide `PORT`/`OPENCODE_PORT` so the health check hits the right port.
- App Platform storage is ephemeral. Workspace, config, and PAM users reset on redeploy unless you add external storage.
- Logs are visible in the App Platform UI without extra setup.
- Provide `OPENCODE_BOOTSTRAP_USER` with either `OPENCODE_BOOTSTRAP_PASSWORD` or `OPENCODE_BOOTSTRAP_PASSWORD_HASH` for first-boot access.
- App Platform supports Linux/AMD64 images and favors smaller image sizes.

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
- Mirror: https://gitea.com/pRizz/opencode-cloud
- Dockerfile: packages/core/src/docker/Dockerfile
