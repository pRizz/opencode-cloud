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
  "public_opencode_url": "${PUBLIC_OPENCODE_DOMAIN_URL}",
  "public_opencode_alb_url": "${PUBLIC_OPENCODE_ALB_URL}",
  "container_username": "${CONTAINER_USERNAME}",
  "container_password": "${CONTAINER_PASSWORD}",
  "host_container_name": "${HOST_CONTAINER_NAME}",
  "host_container_image": "${HOST_CONTAINER_IMAGE}",
  "host_opencode_cloud_cli_version": "${HOST_OPENCODE_CLOUD_CLI_VERSION}"
}
EOF
chmod 600 "${STATUS_FILE}"
opencode_setup_log "opencode-cloud setup: status file written"

opencode_setup_log "opencode-cloud setup: write motd"
secrets_manager_url="https://console.aws.amazon.com/secretsmanager"
if [ -n "${PRIVATE_CREDENTIALS_SECRET_NAME:-}" ] && [ -n "${AWS_REGION:-}" ]; then
  secrets_manager_url="https://console.aws.amazon.com/secretsmanager/secret?name=${PRIVATE_CREDENTIALS_SECRET_NAME}&region=${AWS_REGION}"
elif [ -n "${AWS_REGION:-}" ]; then
  secrets_manager_url="https://console.aws.amazon.com/secretsmanager/home?region=${AWS_REGION}#/secretsmanager"
fi
cat > /etc/motd <<EOF
opencode-cloud ready. (cloud-init)
init logs:
  /var/log/opencode-cloud-setup.log
  /var/log/cloud-init-output.log
  /var/log/cloud-init.log
Container username: ${CONTAINER_USERNAME}
Container password: (redacted)
Public opencode URL: ${PUBLIC_OPENCODE_DOMAIN_URL}
Status file (root-only): ${STATUS_FILE}
Fetch credentials (root-only):
  sudo cat ${STATUS_FILE}
AWS Secrets Manager (if applicable):
  ${secrets_manager_url}
EOF
opencode_setup_log "opencode-cloud setup: motd written"

opencode_setup_mark_provisioned
