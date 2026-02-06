#!/bin/bash
set -euo pipefail

log() {
    echo "[opencode-cloud] $*"
}

read_opencode_cloud_version() {
    local version_file="/etc/opencode-cloud-version"
    local version

    if [ -r "${version_file}" ]; then
        version="$(head -n 1 "${version_file}" 2>/dev/null | tr -d "\r\n")"
    else
        version=""
    fi

    if [ -z "${version}" ]; then
        printf "dev"
    else
        printf "%s" "${version}"
    fi
}

format_url_host() {
    local host="$1"

    if [[ "${host}" == *:* ]] && [[ "${host}" != \[*] ]]; then
        printf "[%s]" "${host}"
    else
        printf "%s" "${host}"
    fi
}

display_local_host() {
    local host="$1"

    if [ "${host}" = "0.0.0.0" ] || [ "${host}" = "::" ]; then
        printf "127.0.0.1"
    else
        printf "%s" "${host}"
    fi
}

build_service_url() {
    local host="$1"
    local port="$2"
    printf "http://%s:%s" "$(format_url_host "${host}")" "${port}"
}

print_welcome_banner() {
    local version local_host local_url bind_url
    version="$(read_opencode_cloud_version)"
    local_host="$(display_local_host "${OPENCODE_HOST}")"
    local_url="$(build_service_url "${local_host}" "${OPENCODE_PORT}")"
    bind_url="$(build_service_url "${OPENCODE_HOST}" "${OPENCODE_PORT}")"

    log "----------------------------------------------------------------------"
    log "Welcome to opencode-cloud-sandbox"
    log "You are running opencode-cloud v${version}"
    log "WARNING: opencode-cloud is still a work in progress and is rapidly evolving."
    log "Expect frequent updates and breaking changes. Use with caution."
    log "For questions, problems, and feature requests, file an issue:"
    log "  https://github.com/pRizz/opencode-cloud/issues"
    log "opencode-cloud runs opencode in a Docker sandbox; use occ/opencode-cloud CLI to manage users, mounts, and updates."
    log ""
    log "Getting started:"
    log "  1) Access the web UI:"
    log "     Local URL: ${local_url}"
    log "     Bind URL:  ${bind_url}"
    log "  2) First-time setup:"
    log "     If no users are configured, this container prints an Initial One-Time Password (IOTP)"
    log "     in the logs below. Enter it on the login page to create your first account."
    log "     The IOTP is deleted immediately after first successful signup."
    log "  3) Optional admin CLI path:"
    log "     occ user add <username>"
    log "     occ user add <username> --generate"
    log "     occ user passwd <username>"
    log "  4) Cloud note: external URL may differ based on DNS, reverse proxy/load balancer,"
    log "     ingress, TLS termination, and port mappings."
    log "  5) Log in with your created credentials. If prompted for optional 2FA setup,"
    log "     you can skip it."
    log "Docs: https://github.com/pRizz/opencode-cloud#readme"
    log "----------------------------------------------------------------------"
}

OPENCODE_PORT="${OPENCODE_PORT:-${PORT:-3000}}"
OPENCODE_HOST="${OPENCODE_HOST:-0.0.0.0}"
export OPENCODE_PORT OPENCODE_HOST

BOOTSTRAP_HELPER="/usr/local/bin/opencode-cloud-bootstrap"
BOOTSTRAP_STATE_DIR="/var/lib/opencode-users"
PROTECTED_USER="opencode"
# NOTE: Do not change this prefix; admins may depend on it for log grep extraction.
greppable_iotp_prefix="INITIAL ONE-TIME PASSWORD (IOTP): "

print_welcome_banner

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
            if [ "${username}" = "${PROTECTED_USER}" ]; then
                log "Skipping protected user record: ${record}"
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
        if [ "${username}" = "${PROTECTED_USER}" ]; then
            log "Skipping persistence for protected user: ${username}"
            return 0
        fi
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

    has_non_protected_persisted_users() {
        shopt -s nullglob
        local records=(/var/lib/opencode-users/*.json)
        local record username
        for record in "${records[@]}"; do
            username="$(jq -r ".username // empty" "${record}" 2>/dev/null || true)"
            if [ -n "${username}" ] && [ "${username}" != "${PROTECTED_USER}" ]; then
                return 0
            fi
        done

        local line home
        while IFS= read -r line; do
            username="$(cut -d: -f1 <<< "${line}")"
            home="$(cut -d: -f6 <<< "${line}")"
            if [[ "${home}" == /home/* ]] && [ "${username}" != "${PROTECTED_USER}" ]; then
                return 0
            fi
        done < <(getent passwd)

        return 1
    }

    clear_bootstrap_state() {
        rm -f \
            "${BOOTSTRAP_STATE_DIR}/.initial-otp.json" \
            "${BOOTSTRAP_STATE_DIR}/.initial-otp.secret" \
            "${BOOTSTRAP_STATE_DIR}/.initial-otp.lock"
    }

    announce_bootstrap_mode() {
        local bootstrap_json="$1"
        local active otp created_at reason
        active="$(jq -r ".active // false" <<< "${bootstrap_json}" 2>/dev/null || true)"
        otp="$(jq -r ".otp // empty" <<< "${bootstrap_json}" 2>/dev/null || true)"
        created_at="$(jq -r ".created_at // empty" <<< "${bootstrap_json}" 2>/dev/null || true)"
        reason="$(jq -r ".reason // empty" <<< "${bootstrap_json}" 2>/dev/null || true)"

        if [ "${active}" = "true" ] && [ -n "${otp}" ]; then
            log "----------------------------------------------------------------------"
            log "${greppable_iotp_prefix}${otp}"
            if [ -n "${created_at}" ]; then
                log "Issued at (UTC): ${created_at}"
            fi
            log "Use this IOTP on the web login page to complete first-time setup."
            log "Find it in Docker logs and keep it private."
            log "This IOTP is deleted after the first successful signup."
            log "----------------------------------------------------------------------"
            return
        fi

        if [ "${reason}" = "user_exists" ]; then
            log "Bootstrap mode disabled: one or more configured users already exist."
            return
        fi

        log "Bootstrap mode unavailable: ${reason:-unknown reason}"
    }

    restore_or_bootstrap_users() {
        if restore_users; then
            log "User records restored"
            return
        fi

        if bootstrap_user; then
            return
        fi

        log "No persisted users and no bootstrap user configured"
    }

    maybe_initialize_bootstrap_mode() {
        local bootstrap_init_json

        if [ ! -x "${BOOTSTRAP_HELPER}" ]; then
            log "Bootstrap helper is missing; first-time setup is unavailable."
            return
        fi

        bootstrap_init_json="$("${BOOTSTRAP_HELPER}" init 2>/dev/null || true)"
        if [ -z "${bootstrap_init_json}" ]; then
            log "Bootstrap helper returned no output; first-time setup may be unavailable."
            return
        fi

        announce_bootstrap_mode "${bootstrap_init_json}"
    }

    sync_bootstrap_state() {
        if has_non_protected_persisted_users; then
            clear_bootstrap_state
            return
        fi

        if [ -n "${OPENCODE_BOOTSTRAP_USER:-}" ]; then
            return
        fi

        maybe_initialize_bootstrap_mode
    }

    restore_or_bootstrap_users
    sync_bootstrap_state

    log "Starting opencode on ${OPENCODE_HOST}:${OPENCODE_PORT}"
    /usr/local/bin/opencode-broker &
    # Use runuser to switch to opencode user without password prompt
    exec runuser -u opencode -- sh -lc "cd /home/opencode/workspace && /opt/opencode/bin/opencode web --port ${OPENCODE_PORT} --hostname ${OPENCODE_HOST}"
fi
