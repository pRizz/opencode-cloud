#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/opencode-cloud-setup.sh"

STATUS_FILE="${OPENCODE_SETUP_STATUS_DIR}/deploy-status.json"

opencode_setup_log "opencode-cloud setup (cloud-init): start"
opencode_setup_set_home

if opencode_setup_is_provisioned; then
  opencode_setup_log "opencode-cloud setup: already provisioned"
  echo "opencode-cloud: already provisioned"
  exit 0
fi

opencode_cloud_setup_run_common

opencode_setup_log "opencode-cloud setup: write status file"
cat > "${STATUS_FILE}" <<EOF
{
  "opencode_url": "${OPENCODE_DOMAIN_URL}",
  "opencode_alb_url": "${OPENCODE_ALB_URL}",
  "username": "${OPENCODE_USERNAME}",
  "password": "${OPENCODE_PASSWORD}",
  "container": "${OPENCODE_CONTAINER_NAME}",
  "image": "${OPENCODE_IMAGE}",
  "cli_version": "${OPENCODE_CLI_VERSION}"
}
EOF
chmod 600 "${STATUS_FILE}"
opencode_setup_log "opencode-cloud setup: status file written"

opencode_setup_log "opencode-cloud setup: write motd"
cat > /etc/motd <<EOF
opencode-cloud ready. (cloud-init)
init logs:
  /var/log/opencode-cloud-setup.log
  /var/log/cloud-init-output.log
  /var/log/cloud-init.log
Username: ${OPENCODE_USERNAME}
Password: ${OPENCODE_PASSWORD}
opencode: ${OPENCODE_DOMAIN_URL}
Status file: ${STATUS_FILE}
EOF
opencode_setup_log "opencode-cloud setup: motd written"

opencode_setup_mark_provisioned
