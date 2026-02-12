# Railway Deployment

Deploy opencode-cloud on Railway as a Docker image service with persistent
storage and automatic HTTPS.

<!-- TODO: Create Railway template and replace TEMPLATE_CODE below.
Steps:
1. Go to Railway dashboard > Templates > New Template
2. Add a service with Docker image: prizz/opencode-cloud-sandbox:latest
3. Add variable: OPENCODE_HOST=0.0.0.0
4. Right-click service > Attach Volume > mount path: /home/opencoder/.local/share/opencode
5. Settings > Public Networking > enable HTTP
6. Create and publish the template
7. Copy the template URL and replace TEMPLATE_CODE below
-->

## One-Click Deploy

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/new/template/TEMPLATE_CODE)

> **Important:** After deploying, verify that a Railway Volume is attached.
> Without a volume, all data is lost on every redeploy.

## Template Base Compose for Railway Import (Manual Upload)

If you are creating a Railway template by importing a Compose file, use
`docker-compose.railway-template-base.yml` from this repository.

Do **not** import the root `docker-compose.yml` for Railway templates. That
file is local/quick-deploy oriented and declares six volumes.

```bash
curl -O https://raw.githubusercontent.com/pRizz/opencode-cloud/main/docker-compose.railway-template-base.yml
```

This file is intentionally a **template-base import artifact**, not a full
runtime compose parity file.

Railway template import currently supports one volume per service and does not
support all compose features uniformly. The template-base file intentionally
keeps:
- one persisted mount at `/home/opencoder/.local/share/opencode`
- a fixed image reference (no env interpolation in `image`)
- an import-focused subset of compose keys, with runtime knobs kept where
  Railway supports them

After import, apply any additional Railway-specific runtime configuration in
Railway settings/template editor.

## Manual Deployment

### Prerequisites

- A [Railway](https://railway.com) account (free tier works for testing)

### 1) Create a new project

In the Railway dashboard, create a new project and add a service:

- Click **New Project** > **Deploy a Docker Image**
- Enter the image: `prizz/opencode-cloud-sandbox:latest`

Prefer a **pinned tag** (like `15.2.0`) for reproducible deployments.
See [Docker Hub](https://hub.docker.com/r/prizz/opencode-cloud-sandbox) for
available tags.

### 2) Configure environment variables

In the service's **Variables** tab, add:

| Variable | Value | Required | Notes |
|----------|-------|----------|-------|
| `OPENCODE_HOST` | `0.0.0.0` | Yes | Binds to all interfaces so Railway can route traffic |
| `OPENCODE_PORT` | `3000` | No | Default is 3000; Railway also sets `PORT` automatically |

Railway automatically sets `PORT` and `RAILWAY_ENVIRONMENT`. The entrypoint
reads `PORT` as a fallback if `OPENCODE_PORT` is not set.

### 3) Attach a Railway Volume (critical)

> **This step prevents data loss.** Without a volume, all session data,
> workspace files, and user accounts are lost when the container is redeployed.

1. Right-click the service > **Attach Volume**
2. Set the **Mount Path** to: `/home/opencoder/.local/share/opencode`
3. Click **Create**

This single mount covers session storage, project data, and application state.

Railway currently supports one volume per service. The mount path above covers
the most critical data. See [Persistence Details](#persistence-details) for
what each path stores.

### 4) Deploy

Railway deploys automatically after configuration changes. Wait for the
deployment to complete (visible in the Deployments tab).

Railway auto-generates an HTTPS URL for the service. Find it in the service's
**Settings** > **Networking** section. Custom domains can also be configured
there.

Startup logs include container-local access hints (`Local URL`, `Bind URL`).
When `RAILWAY_PUBLIC_DOMAIN` is present, logs may also include
`External URL (Railway)`. If you configure a custom domain or reverse proxy,
use that URL even when container-local URLs differ.

### 5) Complete first-time setup

After the first deploy, retrieve the Initial One-Time Password (IOTP) from
the deploy logs:

1. Click the deployment > **View Logs**
2. Search for `INITIAL ONE-TIME PASSWORD (IOTP):`
3. Copy the IOTP value

Open the auto-generated Railway URL and complete setup:

1. Enter the IOTP on the login page first-time setup panel
2. Continue to passkey setup
3. Either enroll a passkey for the default `opencoder` account, or use the
   username/password fallback to create your first managed user

The IOTP is invalidated after successful enrollment.

## Persistence Details

The container uses these paths for persistent data:

| Path | Purpose | Priority |
|------|---------|----------|
| `/home/opencoder/.local/share/opencode` | Session data, project storage, application state | **Critical** |
| `/home/opencoder/workspace` | Project files (working directory) | High |
| `/home/opencoder/.ssh` | SSH keys | High |
| `/var/lib/opencode-users` | User account records (password hashes, lock status) | High |
| `/home/opencoder/.config/opencode` | opencode configuration | Medium |
| `/home/opencoder/.local/state/opencode` | Application state | Medium |
| `/home/opencoder/.cache/opencode` | Cache data | Low |

With Railway's single-volume limitation, mounting
`/home/opencoder/.local/share/opencode` covers the most critical data. Data
in the other paths will be reset on each redeploy unless additional
persistence is configured.

For the full volume reference, backup instructions, and design rationale, see
[Persistence and Data Storage](../persistence.md).

The Docker image declares `VOLUME` directives for all seven paths, which
provides anonymous volume persistence across container **restarts**. However,
anonymous volumes do **not** survive container **recreation** (which is what
Railway does on each redeploy). That is why the explicit Railway Volume is
required.

## Updating

To update to a newer version:

1. Change the image tag in the service settings (e.g., `15.2.0` to `15.3.0`)
2. Railway will automatically redeploy

If using `latest`, trigger a manual redeploy to pull the newest image.

Data in the attached Railway Volume persists across redeployments.

## Troubleshooting

### Data lost after redeploy

**Cause:** No Railway Volume is attached, or it is mounted to the wrong path.

**Fix:** Attach a volume mounted to `/home/opencoder/.local/share/opencode`
as described in step 3 above.

### IOTP not visible in logs

**Cause:** The IOTP is only printed on first startup when no managed users
exist. If users were previously configured (even in a now-lost volume), the
IOTP may not appear.

**Fix:** If the container has no volume and was redeployed fresh, the IOTP
should appear in the deployment logs. Check the full log output, not just
the build logs.

### Container fails to start

**Cause:** Port binding or image pull issues.

**Fix:**
- Ensure `OPENCODE_HOST=0.0.0.0` is set
- Verify the image tag exists on Docker Hub
- Check Railway deployment logs for errors

### `EACCES` creating `/home/opencoder/.local/share/opencode/bin`

**Symptom:** Deploy logs repeatedly show:

```text
EACCES: permission denied, mkdir '/home/opencoder/.local/share/opencode/bin'
```

**Cause:** The Railway volume is mounted with root ownership, but the app runs
as `opencoder`. If the mounted path is not writable by `opencoder`, startup
fails.

**Immediate workaround (Railway service settings):**
1. Ensure `RAILWAY_RUN_UID=0` (or remove a conflicting non-root value).
2. Set the service **Start Command** to:

```bash
/bin/sh -c 'install -d -m 0755 /home/opencoder/.local/share/opencode && chown -R opencoder:opencoder /home/opencoder/.local/share/opencode && exec /usr/local/bin/entrypoint.sh'
```

After redeploy, the `EACCES` lines should stop.

**Long-term behavior:** Recent images auto-check this mount on startup and
attempt to fix ownership before launching opencode. Keep the volume mounted at
`/home/opencoder/.local/share/opencode`.

## Limitations

- **Single volume per service:** Railway currently supports one volume per
  service. Only the most critical path can be persisted via volume; other
  paths reset on redeploy.
- **No `occ` CLI:** The `occ` CLI is not available inside the Railway
  container. User management and configuration are done through the web UI
  or by setting environment variables.
- **No SSH access:** Railway does not provide SSH access to containers.
  Use the Railway dashboard logs for debugging.

## Railway Template Maintenance

To create or update the Railway one-click deploy template:

1. Go to Railway dashboard > **Workspace Settings** > **Templates**
2. Click **New Template** (or edit the existing one)
3. Add a service with Docker image: `prizz/opencode-cloud-sandbox:latest`
4. Configure variables: `OPENCODE_HOST=0.0.0.0`
5. Right-click service > **Attach Volume** > mount path:
   `/home/opencoder/.local/share/opencode`
6. Settings > **Public Networking** > enable HTTP
7. **Create** and **Publish** the template
8. Copy the template URL and update the deploy button in `README.md`

Maintenance rule:
- When image references or required Railway env keys change, update both
  `docker-compose.yml` and `docker-compose.railway-template-base.yml` in the
  same PR.

The deploy button format:

```markdown
[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/new/template/YOUR_TEMPLATE_CODE)
```
