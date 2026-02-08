# DigitalOcean Droplet (Manual Setup)

Manual deployment instructions for running opencode-cloud on a DigitalOcean
Droplet before the Marketplace 1-click image is available.

## Overview

What you get:

- **Persistence**: your workspace, config, and PAM users persist on the Droplet disk.
- **Sandboxing**: opencode runs inside `prizz/opencode-cloud-sandbox` (Docker).

Security posture (recommended):

- Keep the service **bound to localhost**.
- Access it via an **SSH tunnel** (`ssh -L ...`).
- Do **not** expose port `3000` publicly until you have a user, and a firewall/TLS plan.

## Prereqs

- Droplet: **Ubuntu 24.04 LTS**.
- Disk: **50GB+** recommended (Docker images + workspace can grow).
- SSH key added during Droplet creation.

## Path 1 (Recommended): Install `occ` on the Droplet

### 1) Create the Droplet

In the DigitalOcean UI:

- Choose **Ubuntu 24.04 LTS**
- Add your **SSH key**
- Optional: attach a **Cloud Firewall** that only allows inbound `22/tcp` from your IP

### 2) SSH into the Droplet

```bash
ssh root@<droplet-ip>
```

### 3) Install Docker + deps

```bash
apt-get update -y
apt-get install -y docker.io curl jq
systemctl enable --now docker
docker --version
```

If `cargo install` fails later due to missing build dependencies, install:

```bash
apt-get install -y build-essential pkg-config libssl-dev
```

### 4) Install Rust (rustup) + `opencode-cloud`

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y --profile minimal
. "$HOME/.cargo/env"
cargo install opencode-cloud
occ --version
```

### 5) Start the service (localhost-only default)

Pull and start the sandbox image:

```bash
occ start --pull-sandbox-image
```

### 6) Complete first-time setup with Initial One-Time Password (IOTP)

After `occ start`, check logs for the IOTP:

```bash
occ logs
```

Extract just the IOTP value (optional):

```bash
occ logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

If you used `occ setup`, it will also try to auto-detect and print the IOTP after starting/restarting the service.

Open the web login page through your SSH tunnel and use the first-time setup panel:
- Enter the IOTP from logs
- Continue to passkey setup
- Then either:
  - Enroll a passkey for the default `opencoder` account, or
  - Use username/password fallback to create your first managed user

The IOTP is invalidated after successful passkey enrollment or successful username/password bootstrap signup.

Admin fallback:

```bash
occ user add admin --generate
```

### 7) Access via SSH tunnel (recommended default)

From your laptop:

```bash
ssh -L 3000:localhost:3000 root@<droplet-ip>
```

Then open:

- `http://localhost:3000`

Log in with the account you created in the first-time setup panel.

### 8) Enable reboot persistence via systemd (system-level)

```bash
occ config set boot_mode system
occ install
systemctl status opencode-cloud --no-pager
```

View logs:

```bash
journalctl -u opencode-cloud -f
```

### 9) Optional: expose port 3000 publicly (only after user + firewall)

Bind to all interfaces:

```bash
occ config set bind_address 0.0.0.0
occ restart
```

Firewall recommendations:

- Allow inbound `3000/tcp` **only** from your IP (or office/VPN CIDR).
- Keep inbound `22/tcp` restricted.

Confirm unauthenticated access is disabled:

```bash
occ config get allow_unauthenticated_network
```

It should be `false`.

## Optional HTTPS (Caddy)

Goal: keep opencode on localhost, expose only `80/443` publicly.

### 1) Ensure opencode binds to localhost

```bash
occ config set bind_address localhost
occ restart
```

### 2) Install Caddy

```bash
apt-get update -y
apt-get install -y caddy
systemctl enable --now caddy
```

If `apt-get install caddy` fails, install from the official Caddy repo instead.

### 3) Configure Caddy

Edit `/etc/caddy/Caddyfile`:

```caddyfile
your-domain.example.com {
  reverse_proxy 127.0.0.1:3000
}
```

Reload Caddy:

```bash
systemctl reload caddy
```

### 4) DigitalOcean firewall

- Allow inbound `80/tcp` and `443/tcp` from `0.0.0.0/0`
- Keep inbound `3000/tcp` **closed**

### 5) Proxy headers note (trustProxy)

If you are running behind a reverse proxy and need opencode to trust
`X-Forwarded-*` headers, verify `auth.trustProxy` in opencode config and restart.
The current default is `"auto"`; setting `trustProxy: true` explicitly is still a valid override.

This file lives on the host bind-mount. If you ran `occ` as `root`, it is
typically:

- `/root/.config/opencode/opencode.jsonc`

Example config:

```jsonc
{
  "auth": {
    "enabled": true,
    "trustProxy": true
  }
}
```

Then restart:

```bash
occ restart
```

## Path 2: Docker-only (Image Direct)

Use this if you don't want `occ` installed on the Droplet.

> The Docker image declares `VOLUME` directives for all critical paths,
> providing anonymous volume fallback if you forget explicit `-v` flags.
> Named volumes (used by both methods below) are recommended for durable
> persistence.

### Path 2A: Docker Compose (Recommended)

Docker Compose configures all 6 named volumes automatically.

#### 1) Install Docker

```bash
apt-get update -y
apt-get install -y docker.io curl jq
systemctl enable --now docker
```

#### 2) Download docker-compose.yml

```bash
curl -O https://raw.githubusercontent.com/pRizz/opencode-cloud/main/docker-compose.yml
```

#### 3) Start the service

```bash
docker compose up -d
```

#### 4) Retrieve the IOTP

```bash
docker compose logs | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

#### 5) Access via SSH tunnel (recommended default)

From your laptop:

```bash
ssh -L 3000:localhost:3000 root@<droplet-ip>
```

Then open `http://localhost:3000`.

Enter the IOTP on the login page first-time setup panel, then enroll a passkey
for the default `opencoder` account or use username/password fallback.

#### 6) Expose publicly (optional)

Edit `docker-compose.yml` and change the port binding:

```yaml
ports:
  - "3000:3000"  # was "127.0.0.1:3000:3000"
```

Then restart:

```bash
docker compose down && docker compose up -d
```

Firewall recommendations:

- Allow inbound `3000/tcp` **only** from your IP (or office/VPN CIDR).
- Keep inbound `22/tcp` restricted.
- Consider adding HTTPS via Caddy (see [Optional HTTPS](#optional-https-caddy) above).

### Path 2B: Docker CLI (Manual)

Use this if you prefer direct `docker run` commands without Compose.

#### 1) Install Docker

```bash
apt-get update -y
apt-get install -y docker.io curl jq
systemctl enable --now docker
docker --version
```

#### 2) Create Docker volumes

```bash
docker volume create opencode-data
docker volume create opencode-state
docker volume create opencode-cache
docker volume create opencode-workspace
docker volume create opencode-config
docker volume create opencode-users
```

#### 3) Run the container (SSH tunnel default: bind host port to localhost)

```bash
docker run -d --name opencode-cloud-sandbox \
  --restart unless-stopped \
  -p 127.0.0.1:3000:3000 \
  -v opencode-data:/home/opencoder/.local/share/opencode \
  -v opencode-state:/home/opencoder/.local/state/opencode \
  -v opencode-cache:/home/opencoder/.cache/opencode \
  -v opencode-workspace:/home/opencoder/workspace \
  -v opencode-config:/home/opencoder/.config/opencode \
  -v opencode-users:/var/lib/opencode-users \
  prizz/opencode-cloud-sandbox:15.2.0
```

Notes:

- Prefer a **pinned tag** (like `15.2.0`) for reproducible deployments.
- See Docker Hub for tags: https://hub.docker.com/r/prizz/opencode-cloud-sandbox

#### 4) Access via SSH tunnel

From your laptop:

```bash
ssh -L 3000:localhost:3000 root@<droplet-ip>
```

Then open `http://localhost:3000`.

Before signing in, read the container logs and copy the Initial One-Time Password (IOTP):

```bash
docker logs opencode-cloud-sandbox
```

Extract only the IOTP value:

```bash
docker logs opencode-cloud-sandbox 2>&1 | grep -F "INITIAL ONE-TIME PASSWORD (IOTP): " | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'
```

Use the login page first-time setup panel with that IOTP, then continue to passkey setup where you can either enroll a passkey for `opencoder` or choose username/password registration to create your first managed user.

#### 5) Expose publicly (optional)

- Change to `-p 0.0.0.0:3000:3000` (or `-p 3000:3000`)
- Apply a DigitalOcean firewall allowlist (recommended)

## Troubleshooting

Container status/logs:

```bash
docker ps
docker logs opencode-cloud-sandbox
```

CLI status/logs (if using `occ`):

```bash
occ status
occ logs
```

systemd logs (if installed via `occ install`):

```bash
journalctl -u opencode-cloud -f
```
