# Marketplace Validation Scripts

This directory vendors DigitalOcean Marketplace image validation scripts from
`digitalocean/marketplace-partners` so our Marketplace build is reproducible and
does not depend on fetching scripts at build time.

Vendored from:

- Repo: `https://github.com/digitalocean/marketplace-partners`
- Commit: `c83a03d01202fedef5103bb99c7b3a7734149fec`

Files:

- `90-cleanup.sh`
- `99-img-check.sh`

License: see `infra/digitalocean/marketplace/LICENSE`.

## Updating

1. Copy the latest `scripts/90-cleanup.sh` and `scripts/99-img-check.sh` into
   this directory.
2. Copy (or update) the upstream license file into `infra/digitalocean/marketplace/LICENSE`.
3. Update the commit SHA above.
4. Keep upstream headers intact.
