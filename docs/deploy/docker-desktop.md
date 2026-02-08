# Docker Desktop / Docker CLI

Run opencode-cloud-sandbox locally using Docker Desktop (macOS, Windows, Linux)
or the Docker CLI on any machine.

## Overview

What you get:

- **Sandboxed environment**: opencode runs inside the `prizz/opencode-cloud-sandbox` container
- **Local access**: Web UI at `http://localhost:3000`
- **Persistence**: Named volumes preserve your data across container restarts and updates

No cloud account or `occ` CLI installation is required.

## Quick Start (Docker Compose)

The project includes a `docker-compose.yml` that configures all persistent
volumes automatically.

```bash
# Clone or download docker-compose.yml from the repository root
curl -O https://raw.githubusercontent.com/pRizz/opencode-cloud/main/docker-compose.yml

# Start the service
docker compose up -d
```

Retrieve the Initial One-Time Password (IOTP) from logs:

```bash
docker compose logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

Open `http://localhost:3000`, enter the IOTP, and complete first-time setup.

To stop:

```bash
docker compose down
```

To stop and remove volumes (data loss):

```bash
docker compose down -v
```

## Quick Start (Docker CLI)

### 1) Create named volumes

```bash
docker volume create opencode-data
docker volume create opencode-state
docker volume create opencode-cache
docker volume create opencode-workspace
docker volume create opencode-config
docker volume create opencode-users
```

### 2) Run the container

```bash
docker run -d --name opencode-cloud-sandbox \
  --restart unless-stopped \
  -p 127.0.0.1:3000:3000 \
  -e OPENCODE_HOST=0.0.0.0 \
  -v opencode-data:/home/opencoder/.local/share/opencode \
  -v opencode-state:/home/opencoder/.local/state/opencode \
  -v opencode-cache:/home/opencoder/.cache/opencode \
  -v opencode-workspace:/home/opencoder/workspace \
  -v opencode-config:/home/opencoder/.config/opencode \
  -v opencode-users:/var/lib/opencode-users \
  prizz/opencode-cloud-sandbox:latest
```

Notes:

- Prefer a **pinned tag** (like `15.2.0`) for reproducible deployments.
- See Docker Hub for tags: https://hub.docker.com/r/prizz/opencode-cloud-sandbox
- Use `-p 3000:3000` instead of `-p 127.0.0.1:3000:3000` to expose on all
  network interfaces.

### 3) Complete first-time setup

Retrieve the IOTP from logs:

```bash
docker logs opencode-cloud-sandbox 2>&1 | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

Open `http://localhost:3000` and complete setup:

1. Enter the IOTP on the login page first-time setup panel
2. Continue to passkey setup
3. Either enroll a passkey for the default `opencoder` account, or use
   username/password fallback to create your first managed user

The IOTP is invalidated after successful enrollment.

## Quick Start (Docker Desktop GUI)

1. Open Docker Desktop
2. Go to **Images** > search for `prizz/opencode-cloud-sandbox`
3. Pull the image
4. Click **Run** with these optional settings:
   - **Port**: map host port `3000` to container port `3000`
5. The container starts with anonymous volumes for persistent data

> **Note:** Anonymous volumes created by Docker Desktop survive container
> **restarts**, but are deleted if you remove the container. For durable
> persistence, use Docker Compose or the CLI method above with named volumes.

To view logs, click the running container in Docker Desktop and open the
**Logs** tab. Search for `INITIAL ONE-TIME PASSWORD` to find the IOTP.

## Persistence

### Volume Paths

The container uses these paths for persistent data:

| Path | Volume Name | Purpose |
|------|-------------|---------|
| `/home/opencoder/.local/share/opencode` | `opencode-data` | Session data, project storage |
| `/home/opencoder/.local/state/opencode` | `opencode-state` | Application state |
| `/home/opencoder/.cache/opencode` | `opencode-cache` | Cache data |
| `/home/opencoder/workspace` | `opencode-workspace` | Project files |
| `/home/opencoder/.config/opencode` | `opencode-config` | Configuration |
| `/var/lib/opencode-users` | `opencode-users` | User accounts |

### What Survives What

| Scenario | Anonymous volumes | Named volumes |
|----------|-------------------|---------------|
| `docker restart` | Data preserved | Data preserved |
| `docker stop` + `docker start` | Data preserved | Data preserved |
| `docker rm` + `docker run` | **Data lost** | Data preserved |
| Image update (`docker pull` + recreate) | **Data lost** | Data preserved |
| `docker compose down` | Data preserved | Data preserved |
| `docker compose down -v` | **Data lost** | **Data lost** |

**Recommendation:** Always use named volumes (via Docker Compose or `-v` flags)
for data you want to keep.

### Anonymous Volume Fallback

The Docker image declares `VOLUME` directives for all six persistent paths.
If you run the container without explicit `-v` flags (e.g., clicking "Run" in
Docker Desktop), Docker creates anonymous volumes automatically. This protects
against accidental data loss on container restarts, but anonymous volumes are
deleted when the container is removed.

## Environment Variables

| Variable | Default | Notes |
|----------|---------|-------|
| `OPENCODE_HOST` | `0.0.0.0` | Must be `0.0.0.0` for Docker Desktop port forwarding to work |
| `OPENCODE_PORT` | `3000` | Container port for the web UI |

## Updating

To update to a newer image version:

```bash
# Docker Compose
docker compose pull
docker compose up -d

# Docker CLI
docker pull prizz/opencode-cloud-sandbox:latest
docker stop opencode-cloud-sandbox
docker rm opencode-cloud-sandbox
# Re-run with the same docker run command from step 2 above
```

Named volumes persist across this process. Your data is preserved.

## Troubleshooting

### Port 3000 already in use

Another service is using port 3000. Change the host port:

```bash
# Docker Compose: edit docker-compose.yml, change "127.0.0.1:3000:3000" to "127.0.0.1:8080:3000"
# Docker CLI: use -p 127.0.0.1:8080:3000
```

Then access via `http://localhost:8080`.

### Container exits immediately

Check logs for errors:

```bash
docker logs opencode-cloud-sandbox
```

Common causes:
- Missing `OPENCODE_HOST=0.0.0.0` environment variable
- Port conflict (see above)

### Permission errors on volumes (macOS)

Docker Desktop on macOS uses a Linux VM. Volume permissions are managed
inside the VM and generally work without issues. If you encounter permission
errors:

1. Ensure you are not mounting host directories that require special Docker
   Desktop file sharing settings
2. Named volumes (recommended) avoid macOS file sharing issues entirely

### Data lost after container removal

You used anonymous volumes (Docker Desktop GUI "Run" button without
configuring named volumes). Re-create the container using Docker Compose
or the CLI method with named `-v` flags to prevent this in the future.
