#!/bin/bash
set -euo pipefail

log() {
    echo "[opencode-cloud] $*"
}

OPENCODE_PORT="${OPENCODE_PORT:-${PORT:-3000}}"
OPENCODE_HOST="${OPENCODE_HOST:-0.0.0.0}"
export OPENCODE_PORT OPENCODE_HOST

detect_droplet() {
    local hint="${OPENCODE_CLOUD_ENV:-}"
    if [ -n "${hint}" ]; then
        hint="$(printf "%s" "${hint}" | tr "[:upper:]" "[:lower:]")"
        if [[ "${hint}" == *digitalocean* || "${hint}" == *droplet* ]]; then
            return 0
        fi
    fi
    curl -fsS --connect-timeout 1 --max-time 1 http://169.254.169.254/metadata/v1/id >/dev/null 2>&1
}

collect_non_persistent_paths() {
    local -a paths=(
        "/home/opencode/workspace"
        "/home/opencode/.local/share/opencode"
        "/home/opencode/.local/state/opencode"
        "/home/opencode/.config/opencode"
        "/var/lib/opencode-users"
    )
    local -a non_persistent=()
    local fs_type
    for path in "${paths[@]}"; do
        fs_type="$(stat -f -c %T "${path}" 2>/dev/null || true)"
        case "${fs_type}" in
            ""|overlay|overlayfs|tmpfs|ramfs|squashfs)
                non_persistent+=("${path}")
                ;;
        esac
    done
    if [ ${#non_persistent[@]} -eq 0 ]; then
        return 1
    fi
    printf "%s\n" "${non_persistent[@]}"
    return 0
}

non_persistent_paths="$(collect_non_persistent_paths || true)"
if [ -n "${non_persistent_paths}" ]; then
    log "================================================================="
    log "WARNING: Persistence is not configured for one or more paths."
    log "Data loss is likely if the container is recreated or updated."
    log "Non-persistent paths:"
    while IFS= read -r path; do
        log "  - ${path}"
    done <<< "${non_persistent_paths}"
    if detect_droplet; then
        log "Detected DigitalOcean Docker Droplet environment."
        log "By default, Docker Droplets do not configure volumes or persistence."
        log "You will almost certainly lose data if you are not careful."
    fi
    log "Configure persistence: https://github.com/pRizz/opencode-cloud#readme"
    log "================================================================="
fi

log "----------------------------------------------------------------------"
log "If you created this container via opencode-cloud CLI, add users with:"
log "  occ user add   (or: opencode-cloud user add)"
log "Learn more: occ --help (or: opencode-cloud --help)"
log "Docs: https://github.com/pRizz/opencode-cloud#readme"
log "----------------------------------------------------------------------"

if [ "${USE_SYSTEMD:-}" = "1" ]; then
    exec /sbin/init
else
    # Ensure broker socket directory exists
    install -d -m 0755 /run/opencode

    # Ensure user records directory exists (ephemeral unless mounted)
    install -d -m 0700 /var/lib/opencode-users

    restore_users() {
        shopt -s nullglob
        local records=(/var/lib/opencode-users/*.json)
        if [ ${#records[@]} -eq 0 ]; then
            return 1
        fi
        for record in "${records[@]}"; do
            local username password_hash locked
            username="$(jq -r ".username // empty" "${record}")"
            password_hash="$(jq -r ".password_hash // empty" "${record}")"
            locked="$(jq -r ".locked // false" "${record}")"
            if [ -z "${username}" ]; then
                log "Skipping invalid user record: ${record}"
                continue
            fi
            if ! id -u "${username}" >/dev/null 2>&1; then
                log "Creating user: ${username}"
                useradd -m -s /bin/bash "${username}"
            fi
            if [ -n "${password_hash}" ]; then
                usermod -p "${password_hash}" "${username}"
            fi
            if [ "${locked}" = "true" ]; then
                passwd -l "${username}" >/dev/null
            else
                passwd -u "${username}" >/dev/null || true
            fi
            log "Restored user: ${username}"
        done
        return 0
    }

    persist_user_record() {
        local username="$1"
        local shadow_hash
        shadow_hash="$(getent shadow "${username}" | cut -d: -f2)"
        if [ -z "${shadow_hash}" ]; then
            log "Failed to read shadow hash for ${username}"
            return 1
        fi
        local status locked
        status="$(passwd -S "${username}" | tr -s " " | cut -d" " -f2)"
        locked="false"
        if [ "${status}" = "L" ]; then
            locked="true"
        fi
        local record_path="/var/lib/opencode-users/${username}.json"
        umask 077
        jq -n --arg username "${username}" --arg hash "${shadow_hash}" --argjson locked "${locked}" '{username:$username,password_hash:$hash,locked:$locked}' > "${record_path}"
        chmod 600 "${record_path}"
        log "Persisted user record: ${username}"
    }

    bootstrap_user() {
        local username="${OPENCODE_BOOTSTRAP_USER:-}"
        local password="${OPENCODE_BOOTSTRAP_PASSWORD:-}"
        local password_hash="${OPENCODE_BOOTSTRAP_PASSWORD_HASH:-}"
        if [ -z "${username}" ]; then
            return 1
        fi
        if [ -z "${password_hash}" ] && [ -z "${password}" ]; then
            log "OPENCODE_BOOTSTRAP_USER is set but no password or hash provided"
            exit 1
        fi
        if ! id -u "${username}" >/dev/null 2>&1; then
            log "Creating bootstrap user: ${username}"
            useradd -m -s /bin/bash "${username}"
        fi
        if [ -n "${password_hash}" ]; then
            usermod -p "${password_hash}" "${username}"
        else
            echo "${username}:${password}" | chpasswd
        fi
        persist_user_record "${username}"
        log "Bootstrap user ready: ${username}"
        return 0
    }

    if restore_users; then
        log "User records restored"
    else
        if ! bootstrap_user; then
            log "No persisted users and no bootstrap user configured"
        fi
    fi

    log "Starting opencode on ${OPENCODE_HOST}:${OPENCODE_PORT}"
    /usr/local/bin/opencode-broker &
    # Use runuser to switch to opencode user without password prompt
    exec runuser -u opencode -- sh -lc "cd /home/opencode/workspace && /opt/opencode/bin/opencode web --port ${OPENCODE_PORT} --hostname ${OPENCODE_HOST}"
fi
