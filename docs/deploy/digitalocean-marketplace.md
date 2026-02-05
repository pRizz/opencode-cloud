# DigitalOcean Marketplace (1-Click Droplet)

This deployment path packages opencode-cloud as a DigitalOcean Marketplace 1-click
Droplet image. Users deploy from the Marketplace listing and the instance
bootstraps itself on first boot.

> Note: Marketplace listing pending. For manual Droplet setup now, see
> `docs/deploy/digitalocean-droplet.md`.

## Deploy from the Marketplace

1. Click the **Deploy on DigitalOcean** button in the root README.
2. Select a region/size and create the Droplet.
3. Wait for cloud-init to finish (usually a few minutes).
4. Open `http://<droplet-public-ip>:3000`.

> Note: update the Marketplace URL in `README.md` and `packages/core/README.md`
> once the listing is approved and a final slug is assigned.

### Retrieve credentials

Credentials are written to `/var/lib/opencode-cloud/deploy-status.json` (root-only):

```bash
sudo cat /var/lib/opencode-cloud/deploy-status.json
```

Logs:

- `/var/log/opencode-cloud-setup.log`
- `/var/log/cloud-init-output.log`
- `/var/log/cloud-init.log`

## Optional user-data overrides

You can supply cloud-init user-data to override defaults by writing
`/etc/opencode-cloud/stack.env`.

Example:

```yaml
#cloud-config
write_files:
  - path: /etc/opencode-cloud/stack.env
    permissions: "0600"
    content: |
      HOST_CONTAINER_IMAGE=prizz/opencode-cloud-sandbox:15.2.0
      HOST_CONTAINER_NAME=opencode-cloud-sandbox
      CONTAINER_USERNAME=opencode
```

## Building the Marketplace image

The Packer template lives at:

- `infra/digitalocean/packer/opencode-marketplace.pkr.hcl`
- `infra/digitalocean/packer/variables.pkr.hcl`

`variables.pkr.hcl` pins the Marketplace container tag for reproducibility.
Update `container_image` and `application_version` for each release snapshot.

Run:

```bash
packer init infra/digitalocean/packer/opencode-marketplace.pkr.hcl
packer validate -var-file=infra/digitalocean/packer/variables.pkr.hcl \
  infra/digitalocean/packer/opencode-marketplace.pkr.hcl
packer build -var-file=infra/digitalocean/packer/variables.pkr.hcl \
  infra/digitalocean/packer/opencode-marketplace.pkr.hcl
```

Or use the `justfile` helpers:

```bash
just do-marketplace-validate
just do-marketplace-build
```

## Marketplace submission checklist (aligned with `marketplace-partners`)

- [ ] Base image is supported (Ubuntu 24.04 LTS).
- [ ] Build droplet uses smallest practical size (default `s-1vcpu-1gb`) to
      maximize customer plan compatibility.
- [ ] Build droplet DO features disabled: monitoring, IPv6, private networking,
      and DO agent.
- [ ] First-boot automation lives in `/var/lib/cloud/scripts/per-instance/` and
      is numerically prefixed (we use `001-opencode-cloud.sh`).
- [ ] Security updates applied (handled by `90-cleanup.sh`).
- [ ] No SSH keys / host keys / cloud-init instance state in the snapshot
      (handled by `90-cleanup.sh`).
- [ ] Firewall configured (we enable + activate `ufw` during build).
- [ ] DO image check passes (run automatically by Packer via `99-img-check.sh`;
      review WARN/FAIL output in the build logs).
- [ ] MOTD uses `/etc/update-motd.d/99-*` (we write `99-opencode-cloud`).
- [ ] Smoke test: create a droplet from the snapshot, wait for cloud-init,
      verify `deploy-status.json` exists and UI is reachable.

### Marketplace cleanup + validation scripts

This repo vendors DigitalOcean's Marketplace image cleanup + validation scripts
from `digitalocean/marketplace-partners` into `infra/digitalocean/marketplace/`:

- `infra/digitalocean/marketplace/90-cleanup.sh`
- `infra/digitalocean/marketplace/99-img-check.sh`

The Packer build runs these scripts as the final step to ensure the snapshot is
ready for submission. Note that `90-cleanup.sh` zero-fills free disk space, so
the final phase of `packer build` can take several minutes.
