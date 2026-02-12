# Persistence and Data Storage

This document is the authoritative reference for how opencode-cloud persists
data across container restarts, updates, and recreations. It covers the volume
architecture, host paths, backup strategies, and design rationale.

## Two Persistence Modes

opencode-cloud supports two ways to persist container data:

| Mode | When it's used | How data is stored |
|------|---------------|-------------------|
| **Named Docker volumes** | Docker Compose (`docker-compose.yml`), `docker run -v` | Docker manages storage on the host; paths are opaque |
| **Bind mounts** | `occ start` (the CLI) | Host directories are mounted directly into the container |

With **named volumes** (Docker Compose / `docker run`), Docker manages the
storage location. You interact with volumes by name (e.g., `opencode-data`)
rather than by host path.

With **bind mounts** (CLI mode), the CLI creates directories on the host and
mounts them into the container. You can see and back up these directories
directly.

Both modes mount to the same container paths, so the application behaves
identically regardless of which mode you use.

## Volume Reference

The container uses 7 persistent storage locations:

| Volume Name | Container Path | CLI Bind Mount (Host Path) | Contents | Backup Priority |
|-------------|---------------|---------------------------|----------|----------------|
| `opencode-data` | `/home/opencoder/.local/share/opencode` | `~/.local/share/opencode` | Session data, project metadata, SSH key metadata | Critical |
| `opencode-workspace` | `/home/opencoder/workspace` | `~/opencode` | Cloned repositories, project files | Critical |
| `opencode-ssh` | `/home/opencoder/.ssh` | `~/.local/share/opencode-cloud/ssh` | SSH private/public keys, SSH config | High |
| `opencode-users` | `/var/lib/opencode-users` | *(named volume only)* | User account records (password hashes, lock status) | High |
| `opencode-config` | `/home/opencoder/.config/opencode` | `~/.config/opencode-cloud/opencode` | opencode app configuration (auth settings, workspace root) | Medium |
| `opencode-state` | `/home/opencoder/.local/state/opencode` | `~/.local/state/opencode` | Application state, update command files | Medium |
| `opencode-cache` | `/home/opencoder/.cache/opencode` | `~/.cache/opencode` | Cache data (regenerable) | Low |

**Note:** `opencode-users` has no default bind mount in CLI mode. It always
uses a named Docker volume because `/var/lib/opencode-users` is a system path
managed internally by the CLI. Users don't need direct filesystem access to
this data.

## What Survives What

| Scenario | Named Volumes | Anonymous Volumes |
|----------|--------------|-------------------|
| `docker restart` | Preserved | Preserved |
| `docker stop` + `docker start` | Preserved | Preserved |
| `docker rm` + `docker run` | Preserved | **Lost** |
| Image update (`docker pull` + recreate) | Preserved | **Lost** |
| `docker compose down` | Preserved | Preserved |
| `docker compose down -v` | **Lost** | **Lost** |
| `occ update container` | Preserved | N/A (CLI uses bind mounts) |
| `occ reset container` | Preserved | N/A |
| `occ reset container --volumes` | **Lost** | N/A |

**Recommendation:** Always use named volumes (Docker Compose or `-v` flags) or
the CLI with bind mounts for data you want to keep. The Docker image declares
`VOLUME` directives for all 7 paths as an anonymous volume fallback, but
anonymous volumes do not survive container removal.

## Non-Persistent Paths

Any container path **not listed above** is ephemeral. Data written to
unlisted paths is lost when the container is removed or recreated.

The entrypoint detects non-persistent paths at startup and logs a warning if
any critical path lacks persistent storage. Look for
`WARNING: Persistence is not configured` in container logs.

## Backing Up Your Data

### Bind Mount Mode (CLI)

When using the CLI (`occ start`), data lives in regular host directories. Back
them up with any file-level backup tool:

```bash
# Back up all critical data
tar czf opencode-backup-$(date +%Y%m%d).tar.gz \
  ~/.local/share/opencode \
  ~/opencode \
  ~/.local/share/opencode-cloud/ssh \
  ~/.config/opencode-cloud/opencode \
  ~/.local/state/opencode
```

The `opencode-users` volume is managed by Docker even in CLI mode. To back it
up, use the named volume method below.

### Named Volume Mode (Docker Compose / `docker run`)

Docker named volumes are not directly accessible as host directories. Use a
temporary container to extract the data:

```bash
# Back up a single volume
docker run --rm \
  -v opencode-data:/data:ro \
  -v "$(pwd)":/backup \
  alpine tar czf /backup/opencode-data.tar.gz -C /data .

# Back up all 7 volumes
for vol in opencode-data opencode-workspace opencode-ssh \
           opencode-users opencode-config opencode-state \
           opencode-cache; do
  docker run --rm \
    -v "${vol}":/data:ro \
    -v "$(pwd)":/backup \
    alpine tar czf "/backup/${vol}.tar.gz" -C /data .
done
```

### Restoring from Backup

**Bind mount mode:** Extract the archive back to the original host paths:

```bash
tar xzf opencode-backup-20250101.tar.gz -C /
```

**Named volume mode:** Stop the container first, then restore:

```bash
docker compose down  # or: docker stop opencode-cloud-sandbox

for vol in opencode-data opencode-workspace opencode-ssh \
           opencode-users opencode-config opencode-state \
           opencode-cache; do
  docker run --rm \
    -v "${vol}":/data \
    -v "$(pwd)":/backup \
    alpine sh -c "rm -rf /data/* && tar xzf /backup/${vol}.tar.gz -C /data"
done

docker compose up -d  # or: docker start opencode-cloud-sandbox
```

## Design Rationale

### Why `~/opencode` for the workspace host path

Upstream opencode's default `workspace.root` is `~/opencode`. By using the
same host path for the CLI bind mount, users who previously ran vanilla
upstream opencode locally will see their existing repos immediately when
switching to opencode-cloud. The host `~/opencode/` maps to the container's
`/home/opencoder/workspace/`, and the container's `workspace.root` config
points there, so clones land on the persistent mount.

### Why SSH keys are isolated from the host's `~/.ssh`

The default CLI bind mount for SSH is `~/.local/share/opencode-cloud/ssh`
rather than the host's `~/.ssh` directory. This is intentional:

1. **Security.** The container runs AI-generated code. If that code (or a
   malicious package) reads files in the mounted SSH directory, an isolated
   path limits exposure to only keys generated inside the sandbox rather than
   the user's entire SSH keyring (which may include keys for production
   servers, infrastructure, etc.).

2. **Write safety.** The opencode app writes to `~/.ssh/config` and creates
   keys under `~/.ssh/opencode/`. If the container modified the host's real
   `~/.ssh/config`, it could break the user's existing SSH setup. SSH config
   parsing is order-sensitive; an opencode-added `Host *` block could shadow
   existing entries.

3. **Permission compatibility.** SSH enforces strict permissions (`~/.ssh`
   must be 700, keys must be 600, owned by the correct user). The container
   runs as `opencoder` (UID 1000), but the host user may have a different
   UID. A bind mount preserves host UID ownership, so the container process
   can't read/write the files unless UIDs happen to match. This would
   silently fail on many setups.

4. **macOS Docker Desktop.** Docker Desktop's Linux VM emulates file
   permissions for bind mounts inconsistently across versions, making host
   `~/.ssh` mounts unreliable for SSH's strict permission checks.

**Power-user escape hatch:** If you want the container to use your host SSH
keys directly, you can override the mount:

```bash
occ mount add ~/.ssh:/home/opencoder/.ssh
occ restart
```

This replaces the default isolated mount with a direct bind to your host's
`~/.ssh`. Use this if you understand the security implications and want
seamless access to your existing keys.

### Why the config host path is `~/.config/opencode-cloud/opencode`

The config bind mount uses `~/.config/opencode-cloud/opencode` rather than
`~/.config/opencode` (the upstream opencode config directory). This prevents
the container's entrypoint from modifying an upstream opencode installation's
config if the user runs both locally. The entrypoint patches config files
(enabling auth, setting `workspace.root`), and writing those changes to
`~/.config/opencode` would alter the user's standalone opencode setup. The
`opencode-cloud` subdirectory keeps our fork's config isolated, consistent
with other fork-owned host paths (`~/.config/opencode-cloud/config.json` for
CLI config, `~/.local/share/opencode-cloud/ssh` for SSH keys).

### Why `opencode-users` has no default bind mount

The `/var/lib/opencode-users` path is a system-level directory managed
internally by the CLI for user account records (JSON files with password
hashes and lock status). It uses a named Docker volume rather than a bind
mount because there is no meaningful host-side equivalent, and the CLI manages
these records programmatically.

## Platform-Specific Notes

### Railway

Railway currently supports one volume per service. Mount it at
`/home/opencoder/.local/share/opencode` to cover the most critical data.
Other paths reset on each redeploy. See [Railway deployment guide](deploy/railway.md)
for details.

### Docker Desktop (GUI)

Clicking "Run" in Docker Desktop without configuring named volumes creates
anonymous volumes. These survive container restarts but are lost if the
container is removed. For durable persistence, use Docker Compose or the CLI
method with named `-v` flags. See [Docker Desktop guide](deploy/docker-desktop.md).

### Isolated Sandbox Instances

When using `--sandbox-instance` (worktree isolation), volume names are
suffixed with the instance ID (e.g., `opencode-data-mytree`,
`opencode-ssh-mytree`). Each instance has its own independent set of 7
volumes.
