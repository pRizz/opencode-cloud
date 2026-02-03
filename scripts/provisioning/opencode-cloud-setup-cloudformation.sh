#!/usr/bin/env bash
set -euo pipefail

OPENCODE_SETUP_USER="ubuntu"
OPENCODE_SETUP_HOME="/home/ubuntu"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/opencode-cloud-setup.sh"

STACK_NAME="${OPENCODE_STACK_NAME:-}"
SIGNAL_RESOURCE="${OPENCODE_SIGNAL_RESOURCE:-OpencodeInstance}"
IMDS_BASE="http://169.254.169.254"

signal_result() {
  local -r exit_code="$1"
  local -r reason="$2"
  local status="SUCCESS"

  if [ "$exit_code" -ne 0 ]; then
    status="FAILURE"
  fi

  if ! command -v aws >/dev/null 2>&1; then
    opencode_setup_log "opencode-cloud setup: aws CLI unavailable for signal"
    if [ "$exit_code" -ne 0 ]; then
      exit "$exit_code"
    fi
    return 0
  fi

  opencode_setup_log "opencode-cloud setup: request instance metadata token"
  token="$(curl -sS -X PUT "$IMDS_BASE/latest/api/token" \
    -H "X-aws-ec2-metadata-token-ttl-seconds: 21600" || true)"
  if [ -n "$token" ]; then
    opencode_setup_log "opencode-cloud setup: fetch instance id with token"
    instance_id="$(curl -sS -H "X-aws-ec2-metadata-token: $token" \
      "$IMDS_BASE/latest/meta-data/instance-id" || true)"
  else
    opencode_setup_log "opencode-cloud setup: fetch instance id without token"
    instance_id="$(curl -sS "$IMDS_BASE/latest/meta-data/instance-id" || true)"
  fi

  if [ -z "$instance_id" ]; then
    opencode_setup_log "opencode-cloud setup: missing instance id for signal"
    if [ "$exit_code" -ne 0 ]; then
      exit "$exit_code"
    fi
    return 0
  fi

  if [ "$exit_code" -ne 0 ]; then
    opencode_setup_log "opencode-cloud setup: delaying failure signal 1200s (reason: $reason)"
    sleep 1200
  fi

  opencode_setup_log "opencode-cloud setup: send cloudformation signal (reason: $reason)"
  aws cloudformation signal-resource \
    --stack-name "$STACK_NAME" \
    --logical-resource-id "$SIGNAL_RESOURCE" \
    --status "$status" \
    --unique-id "$instance_id" \
    --region "${AWS_REGION}" || true
  opencode_setup_log "opencode-cloud setup: cloudformation signal sent"

  if [ "$exit_code" -ne 0 ]; then
    exit "$exit_code"
  fi
}

trap 'signal_result 1 "opencode-cloud bootstrap failed"' ERR

opencode_setup_log "opencode-cloud setup (cloudformation): start"
opencode_setup_set_home

if opencode_setup_is_provisioned; then
  opencode_setup_log "opencode-cloud setup: already provisioned"
  echo "opencode-cloud: already provisioned"
  signal_result 0 "opencode-cloud already provisioned"
  exit 0
fi

opencode_setup_prepare_status_dir
opencode_setup_load_stack_env
opencode_setup_apply_defaults

if [ -z "$PUBLIC_OPENCODE_DOMAIN_URL" ]; then
  opencode_setup_log "opencode-cloud setup: missing PUBLIC_OPENCODE_DOMAIN_URL"
  echo "opencode-cloud: missing PUBLIC_OPENCODE_DOMAIN_URL"
  signal_result 1 "opencode-cloud missing PUBLIC_OPENCODE_DOMAIN_URL"
  exit 1
fi

if [ -z "$PUBLIC_OPENCODE_ALB_URL" ]; then
  opencode_setup_log "opencode-cloud setup: missing PUBLIC_OPENCODE_ALB_URL"
  echo "opencode-cloud: missing PUBLIC_OPENCODE_ALB_URL"
  signal_result 1 "opencode-cloud missing PUBLIC_OPENCODE_ALB_URL"
  exit 1
fi

if [ -z "${OPENCODE_CREDENTIALS_SECRET_ARN:-}" ]; then
  opencode_setup_log "opencode-cloud setup: missing credentials secret ARN"
  echo "opencode-cloud: missing credentials secret ARN"
  signal_result 1 "opencode-cloud missing credentials secret ARN"
  exit 1
fi

if [ -z "${AWS_REGION:-}" ]; then
  opencode_setup_log "opencode-cloud setup: missing AWS region"
  echo "opencode-cloud: missing AWS region"
  signal_result 1 "opencode-cloud missing AWS region"
  exit 1
fi

if [ -z "$STACK_NAME" ]; then
  opencode_setup_log "opencode-cloud setup: missing stack name for signal"
  signal_result 1 "opencode-cloud missing stack name"
  exit 1
fi

if ! command -v aws >/dev/null 2>&1; then
  opencode_setup_log "opencode-cloud setup: install awscli"
  apt-get update -y
  opencode_setup_log "opencode-cloud setup: apt-get updated"
  apt-get install -y snapd
  opencode_setup_log "opencode-cloud setup: snapd installed"
  snap install aws-cli --classic
  opencode_setup_log "opencode-cloud setup: awscli installed via snap"
fi

opencode_setup_configure_rustup_profile
opencode_setup_ensure_rust_toolchain
opencode_setup_ensure_cli
opencode_setup_enable_docker
opencode_setup_wait_for_docker
opencode_setup_bootstrap_config

opencode_setup_log "opencode-cloud setup: install service (ubuntu user)"
opencode_setup_run_as_user "opencode-cloud install --force"
opencode_setup_log "opencode-cloud setup: service install complete (ubuntu user)"

opencode_setup_log "opencode-cloud setup: align host mount ownership"
ubuntu_home="$OPENCODE_SETUP_HOME"
data_dir="$ubuntu_home/.local/share/opencode"
state_dir="$ubuntu_home/.local/state/opencode"
cache_dir="$ubuntu_home/.cache/opencode"
config_dir="$ubuntu_home/.config/opencode"
workspace_dir="$data_dir/workspace"
mkdir -p "$data_dir" "$state_dir" "$cache_dir" "$config_dir" "$workspace_dir"
opencode_uid="$(docker run --rm --entrypoint id "$HOST_CONTAINER_IMAGE" -u opencode)"
opencode_gid="$(docker run --rm --entrypoint id "$HOST_CONTAINER_IMAGE" -g opencode)"
chown -R "$opencode_uid:$opencode_gid" \
  "$data_dir" \
  "$state_dir" \
  "$cache_dir" \
  "$config_dir" \
  "$workspace_dir"
opencode_setup_log "opencode-cloud setup: host mount ownership aligned"

opencode_setup_log "opencode-cloud setup: restart container after mount ownership update"
opencode_setup_run_as_user "opencode-cloud restart --quiet"
opencode_setup_log "opencode-cloud setup: container restart complete"

opencode_setup_log "opencode-cloud setup: wait for service readiness (30 attempts)"
for attempt in $(seq 1 30); do
  opencode_setup_log "opencode-cloud setup: readiness check attempt $attempt/30"
  if curl -sS --max-time 2 http://localhost:3000/ -o /dev/null; then
    opencode_setup_log "opencode-cloud setup: service responded"
    break
  fi
  sleep 2
done

if ! curl -sS --max-time 2 http://localhost:3000/ -o /dev/null; then
  opencode_setup_log "opencode-cloud setup: service not reachable"
  signal_result 1 "opencode-cloud service did not become reachable on port 3000"
fi

opencode_setup_create_user
opencode_setup_disable_unauth_network

opencode_setup_log "opencode-cloud setup: write secrets payload"
if ! command -v jq >/dev/null 2>&1; then
  opencode_setup_log "opencode-cloud setup: jq missing"
  signal_result 1 "jq not available to build secrets payload"
  exit 1
fi

secret_payload="$(jq -n \
  --arg public_opencode_url "$PUBLIC_OPENCODE_DOMAIN_URL" \
  --arg public_opencode_alb_url "$PUBLIC_OPENCODE_ALB_URL" \
  --arg container_username "$CONTAINER_USERNAME" \
  --arg container_password "$CONTAINER_PASSWORD" \
  --arg host_container_name "$HOST_CONTAINER_NAME" \
  --arg host_container_image "$HOST_CONTAINER_IMAGE" \
  '{public_opencode_url:$public_opencode_url,public_opencode_alb_url:$public_opencode_alb_url,container_username:$container_username,container_password:$container_password,host_container_name:$host_container_name,host_container_image:$host_container_image}')"

opencode_setup_log "opencode-cloud setup: store secret"
aws secretsmanager put-secret-value \
  --region "$AWS_REGION" \
  --secret-id "$OPENCODE_CREDENTIALS_SECRET_ARN" \
  --secret-string "$secret_payload"
opencode_setup_log "opencode-cloud setup: secret stored"

opencode_setup_mark_provisioned
signal_result 0 "opencode-cloud provisioned successfully"
