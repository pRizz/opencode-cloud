#!/usr/bin/env bash
set -euo pipefail

STACK_ENV="/etc/opencode-cloud/stack.env"

if [ ! -f "$STACK_ENV" ]; then
  install -d -m 0700 /etc/opencode-cloud
  cat > "$STACK_ENV" <<EOF_STACK
# Keep in sync with infra/digitalocean/packer/variables.pkr.hcl
HOST_CONTAINER_IMAGE=prizz/opencode-cloud-sandbox:15.2.0
HOST_CONTAINER_NAME=opencode-cloud-sandbox
CONTAINER_USERNAME=opencode
OPENCODE_CLOUD_ENV=digitalocean_docker_droplet
PUBLIC_OPENCODE_DOMAIN_URL=
PUBLIC_OPENCODE_ALB_URL=
EOF_STACK
  chmod 0600 "$STACK_ENV"
fi

exec /usr/local/bin/opencode-cloud-setup-digitalocean.sh
