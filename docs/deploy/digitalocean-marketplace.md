# DigitalOcean Marketplace (1-Click Droplet)

This deployment path packages opencode-cloud as a DigitalOcean Marketplace 1-click
Droplet image. Users deploy from the Marketplace listing and the instance
bootstraps itself on first boot.

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

### Marketplace validation scripts

DigitalOcean provides `img_check.sh` and `cleanup.sh` in their Marketplace
partners repo. Copy them into `infra/digitalocean/marketplace/` and run them
against your built snapshot before submitting the image.
