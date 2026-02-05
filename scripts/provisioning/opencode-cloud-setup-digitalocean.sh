#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/opencode-cloud-setup.sh"

STATUS_FILE="${OPENCODE_SETUP_STATUS_DIR}/deploy-status.json"

opencode_setup_log "opencode-cloud setup (digitalocean): start"
opencode_setup_set_home

if opencode_setup_is_provisioned; then
  opencode_setup_log "opencode-cloud setup: already provisioned"
  echo "opencode-cloud: already provisioned"
  exit 0
fi

opencode_cloud_setup_run_common

opencode_setup_log "opencode-cloud setup: write status file"
cat > "${STATUS_FILE}" <<EOF_STATUS
{
  "container_username": "${CONTAINER_USERNAME}",
  "container_password": "${CONTAINER_PASSWORD}",
  "host_container_name": "${HOST_CONTAINER_NAME}",
  "host_container_image": "${HOST_CONTAINER_IMAGE}",
  "host_opencode_cloud_cli_version": "${HOST_OPENCODE_CLOUD_CLI_VERSION}"
}
EOF_STATUS
chmod 600 "${STATUS_FILE}"
opencode_setup_log "opencode-cloud setup: status file written"

opencode_setup_log "opencode-cloud setup: write motd"
install -d -m 0755 /etc/update-motd.d
cat > /etc/update-motd.d/99-opencode-cloud <<EOF_MOTD
#!/usr/bin/env bash
cat <<EOF
opencode-cloud ready. (digitalocean)
init logs:
  /var/log/opencode-cloud-setup.log
  /var/log/cloud-init-output.log
  /var/log/cloud-init.log
Container username: ${CONTAINER_USERNAME}
Container password: (redacted)
Status file (root-only): ${STATUS_FILE}
Fetch credentials (root-only):
  sudo cat ${STATUS_FILE}

Access:
  http://<droplet-public-ip>:3000
EOF
EOF_MOTD
chmod 0755 /etc/update-motd.d/99-opencode-cloud
opencode_setup_log "opencode-cloud setup: motd written"

opencode_setup_mark_provisioned
