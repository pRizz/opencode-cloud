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
        printf "localhost"
    else
        if [ "${host}" = "127.0.0.1" ] || [ "${host}" = "::1" ]; then
            printf "localhost"
            return
        fi
        printf "%s" "${host}"
    fi
}

build_service_url() {
    local host="$1"
    local port="$2"
    printf "http://%s:%s" "$(format_url_host "${host}")" "${port}"
}

railway_external_url() {
    local domain
    domain="${RAILWAY_PUBLIC_DOMAIN:-}"
    domain="$(printf "%s" "${domain}" | tr -d '\r\n' | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')"
    domain="${domain#http://}"
    domain="${domain#https://}"

    while [ "${domain}" != "${domain%/}" ]; do
        domain="${domain%/}"
    done

    if [ -z "${domain}" ]; then
        return 1
    fi

    printf "https://%s" "${domain}"
}

print_welcome_banner() {
    local version local_host local_url bind_url external_url
    version="$(read_opencode_cloud_version)"
    local_host="$(display_local_host "${OPENCODE_HOST}")"
    local_url="$(build_service_url "${local_host}" "${OPENCODE_PORT}")"
    bind_url="$(build_service_url "${OPENCODE_HOST}" "${OPENCODE_PORT}")"
    external_url="$(railway_external_url || true)"

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
    if [ -n "${external_url}" ]; then
        log "     External URL (Railway): ${external_url}"
    fi
    log "     Reverse-proxy/custom-domain URL is also valid when configured."
    log "     Container startup cannot reliably detect proxy/ingress URL unless platform exposes it."
    log "  2) First-time setup:"
    log "     If no users are configured, this container prints an Initial One-Time Password (IOTP)"
    log "     in the logs below. Enter it on the login page, then enroll a passkey"
    log "     for the default 'opencoder' account."
    log "     The IOTP is deleted immediately after successful passkey enrollment."
    log "  3) Optional admin CLI path:"
    log "     occ user add <username>"
    log "     occ user add <username> --generate"
    log "     occ user passwd <username>"
    log "  4) Cloud note: external URL may differ based on DNS, reverse proxy/load balancer,"
    log "     ingress, TLS termination, and port mappings."
    log "  5) Sign in with a passkey (recommended) or username/password fallback."
    log "     2FA setup and management are available from the upper-right session menu."
    log "Docs: https://github.com/pRizz/opencode-cloud#readme"
    log "Deploy guides: https://github.com/pRizz/opencode-cloud/tree/main/docs/deploy"
    log "----------------------------------------------------------------------"
}

OPENCODE_PORT="${OPENCODE_PORT:-${PORT:-3000}}"
OPENCODE_HOST="${OPENCODE_HOST:-0.0.0.0}"
export OPENCODE_PORT OPENCODE_HOST

BOOTSTRAP_HELPER="/usr/local/bin/opencode-cloud-bootstrap"
BOOTSTRAP_STATE_DIR="/var/lib/opencode-users"
PROTECTED_USER="opencoder"
BUILTIN_USERS_FILE="/etc/opencode-cloud/builtin-home-users.txt"
FALLBACK_BUILTIN_HOME_USERS=("opencoder" "ubuntu")
BUILTIN_HOME_USERS=()
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

detect_railway() {
    [ -n "${RAILWAY_ENVIRONMENT:-}" ] && return 0
    [ -n "${RAILWAY_SERVICE_NAME:-}" ] && return 0
    return 1
}

collect_non_persistent_paths() {
    local -a paths=(
        "/home/opencoder/workspace"
        "/home/opencoder/.local/share/opencode"
        "/home/opencoder/.local/state/opencode"
        "/home/opencoder/.cache/opencode"
        "/home/opencoder/.config/opencode"
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
    elif detect_railway; then
        log "Detected Railway environment."
        log "Railway does not automatically configure persistent storage for Docker containers."
        log "Attach a Railway Volume to persist data across deploys."
        log "Mount it to /home/opencoder/.local/share/opencode for session and project data."
        log "Railway deploy guide: https://github.com/pRizz/opencode-cloud/blob/main/docs/deploy/railway.md"
    fi
    log "Configure persistence: https://github.com/pRizz/opencode-cloud/tree/main/docs/deploy"
    log "For docker run, add volume flags such as:"
    log "  -v opencode-data:/home/opencoder/.local/share/opencode"
    log "  -v opencode-workspace:/home/opencoder/workspace"
    log "  -v opencode-users:/var/lib/opencode-users"
    log "  -v opencode-config:/home/opencoder/.config/opencode"
    log "  -v opencode-state:/home/opencoder/.local/state/opencode"
    log "  -v opencode-cache:/home/opencoder/.cache/opencode"
    log "================================================================="
fi

if [ "${USE_SYSTEMD:-}" = "1" ]; then
    exec /sbin/init
else
    # Ensure broker socket directory exists
    install -d -m 0755 /run/opencode

    # Ensure user records directory exists (ephemeral unless mounted)
    install -d -m 0700 /var/lib/opencode-users

    load_builtin_home_users() {
        BUILTIN_HOME_USERS=("${FALLBACK_BUILTIN_HOME_USERS[@]}")

        if [ ! -r "${BUILTIN_USERS_FILE}" ]; then
            return 0
        fi

        local username
        while IFS= read -r username; do
            username="$(printf "%s" "${username}" | tr -d '\r\n')"
            if [ -n "${username}" ]; then
                BUILTIN_HOME_USERS+=("${username}")
            fi
        done < "${BUILTIN_USERS_FILE}"
    }

    is_builtin_home_user() {
        local username="$1"
        local builtin
        for builtin in "${BUILTIN_HOME_USERS[@]}"; do
            if [ "${username}" = "${builtin}" ]; then
                return 0
            fi
        done
        return 1
    }

    user_record_path() {
        local username="$1"
        printf "/var/lib/opencode-users/%s.json" "${username}"
    }

    user_record_exists() {
        local username="$1"
        [ -f "$(user_record_path "${username}")" ]
    }

    ensure_jsonc_parser() {
        if ! command -v jq >/dev/null 2>&1; then
            log "ERROR: jq is required to parse JSONC configs."
            return 1
        fi
        return 0
    }

    jsonc_get_auth_enabled() {
        local file="$1"
        ensure_jsonc_parser || return 1

        local auth_enabled
        if ! auth_enabled="$(grep -v '^\s*//' "${file}" | jq -r '.auth.enabled // false')"; then
            return 1
        fi
        printf '%s' "${auth_enabled}"
    }

    jsonc_set_auth_enabled() {
        local file="$1"
        ensure_jsonc_parser || return 1

        local patched
        if ! patched="$(grep -v '^\s*//' "${file}" | jq '.auth.enabled = true')"; then
            return 1
        fi
        printf '%s\n' "${patched}" > "${file}"
    }

    ensure_auth_config() {
        local config_dir="/home/opencoder/.config/opencode"
        local config_json="${config_dir}/opencode.json"
        local config_jsonc="${config_dir}/opencode.jsonc"

        install -d -m 0755 "${config_dir}"

        # Check if an existing config already has auth enabled
        local config_file=""
        for candidate in "${config_json}" "${config_jsonc}" "${config_dir}/config.json"; do
            if [ -f "${candidate}" ]; then
                config_file="${candidate}"
                break
            fi
        done

        if [ -n "${config_file}" ]; then
            # File exists — verify auth is enabled
            local auth_enabled
            if ! auth_enabled="$(jsonc_get_auth_enabled "${config_file}")"; then
                log "ERROR: Failed to parse ${config_file} for auth settings."
                exit 1
            fi
            if [ "${auth_enabled}" = "true" ]; then
                return  # Already configured correctly
            fi

            # Auth not enabled — patch the existing config to enable it
            log "Auth is not enabled in ${config_file}; patching to enable."
            if ! jsonc_set_auth_enabled "${config_file}"; then
                log "ERROR: Failed to update ${config_file} to enable auth."
                exit 1
            fi
            chown opencoder:opencoder "${config_file}" 2>/dev/null || true
            chmod 644 "${config_file}" 2>/dev/null || true
            return
        fi

        # No config file — create default
        if ! cat > "${config_jsonc}" <<'EOF'
{
  "auth": {
    "enabled": true
  }
}
EOF
        then
            log "WARNING: Failed to create ${config_jsonc}; auth may be disabled."
            return
        fi

        chown opencoder:opencoder "${config_jsonc}" 2>/dev/null || true
        chmod 644 "${config_jsonc}" 2>/dev/null || true
        log "Created default auth config at ${config_jsonc}."
    }

    check_auth_enabled() {
        local config_dir="/home/opencoder/.config/opencode"

        # Check all config files that opencode's global loader merges
        local candidate auth_enabled
        for candidate in "${config_dir}/opencode.json" \
                         "${config_dir}/opencode.jsonc" \
                         "${config_dir}/config.json"; do
            if [ -f "${candidate}" ]; then
                if ! auth_enabled="$(jsonc_get_auth_enabled "${candidate}")"; then
                    log "ERROR: Failed to parse ${candidate} for auth settings."
                    exit 1
                fi
                if [ "${auth_enabled}" = "true" ]; then
                    return 0
                fi
            fi
        done

        return 1
    }

    warn_security_posture() {
        if ! check_auth_enabled; then
            log "================================================================="
            log "SECURITY WARNING: Authentication is not enabled."
            log "Anyone who can reach this container can use opencode without signing in."
            log "Enable auth by creating or editing the opencode config:"
            log "  /home/opencoder/.config/opencode/opencode.jsonc"
            log '  { "auth": { "enabled": true } }'
            log "Docs: https://github.com/pRizz/opencode-cloud/tree/main/docs/deploy"
            log "================================================================="
        fi

        # Advisory HTTPS notice for cloud deployments binding to all interfaces
        if [ "${OPENCODE_HOST}" = "0.0.0.0" ] || [ "${OPENCODE_HOST}" = "::" ]; then
            if detect_droplet; then
                log "================================================================="
                log "SECURITY NOTICE: opencode is binding to all network interfaces."
                log "If this container is exposed to the internet without HTTPS,"
                log "credentials and session data will be transmitted in the clear."
                log ""
                log "Recommended: terminate TLS in front of opencode."
                log "  - Reverse proxy (Caddy, Nginx, Traefik)"
                log "  - Cloud load balancer with TLS"
                log "Caddy setup: https://github.com/pRizz/opencode-cloud/blob/main/docs/deploy/digitalocean-droplet.md#optional-https-caddy"
                log "================================================================="
            fi
        fi
    }

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
            if is_builtin_home_user "${username}"; then
                log "Skipping built-in user record: ${record}"
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
        if is_builtin_home_user "${username}"; then
            log "Skipping persistence for built-in user: ${username}"
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
        local record_path
        record_path="$(user_record_path "${username}")"
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
        # Persisted records are the source of truth for managed login users.
        # Built-in image users (e.g., ubuntu/opencoder) must not disable IOTP.
        shopt -s nullglob
        local records=(/var/lib/opencode-users/*.json)
        local record username
        for record in "${records[@]}"; do
            username="$(jq -r ".username // empty" "${record}" 2>/dev/null || true)"
            if [ -z "${username}" ]; then
                continue
            fi
            if is_builtin_home_user "${username}"; then
                continue
            fi
            if [ "${username}" != "${PROTECTED_USER}" ]; then
                return 0
            fi
        done

        return 1
    }

    migrate_unmanaged_home_users_to_records() {
        # Migrate any non-built-in Linux users before bootstrap checks so real
        # manually-created accounts disable IOTP in a consistent, managed way.
        local line username home
        while IFS= read -r line; do
            username="$(cut -d: -f1 <<< "${line}")"
            home="$(cut -d: -f6 <<< "${line}")"
            if [[ "${home}" != /home/* ]]; then
                continue
            fi
            if is_builtin_home_user "${username}"; then
                continue
            fi
            if user_record_exists "${username}"; then
                continue
            fi
            if persist_user_record "${username}"; then
                log "Migrated unmanaged user to managed records: ${username}"
            else
                log "WARNING: Failed to migrate unmanaged user to records: ${username}"
            fi
        done < <(getent passwd)
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
            log "This IOTP is deleted after successful passkey enrollment."
            log "----------------------------------------------------------------------"
            return
        fi

        if [ "${reason}" = "user_exists" ]; then
            log "Bootstrap mode disabled: one or more configured users already exist."
            return
        fi

        if [ "${reason}" = "completed" ]; then
            log "Bootstrap mode disabled: initial passkey setup for '${PROTECTED_USER}' is already complete."
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

    ensure_opencode_data_dir_writable() {
        local data_dir="/home/opencoder/.local/share/opencode"

        install -d -m 0755 "${data_dir}"

        if runuser -u opencoder -- test -w "${data_dir}"; then
            return
        fi

        log "Detected non-writable opencode data directory; attempting ownership fix: ${data_dir}"
        if ! chown -R opencoder:opencoder "${data_dir}" 2>/dev/null; then
            log "WARNING: Failed to change ownership for ${data_dir}; continuing with writability re-check."
        fi

        if runuser -u opencoder -- test -w "${data_dir}"; then
            return
        fi

        log "ERROR: ${data_dir} is not writable by user 'opencoder'."
        log "If running on Railway, set RAILWAY_RUN_UID=0 and attach a volume mounted at ${data_dir}."
        exit 1
    }

    load_builtin_home_users
    if ! ensure_jsonc_parser; then
        log "ERROR: JSONC parser is required for auth config checks."
        exit 1
    fi
    ensure_auth_config
    restore_or_bootstrap_users
    migrate_unmanaged_home_users_to_records
    sync_bootstrap_state
    warn_security_posture
    ensure_opencode_data_dir_writable

    log "Starting opencode on ${OPENCODE_HOST}:${OPENCODE_PORT}"
    /usr/local/bin/opencode-broker &
    # Use runuser to switch to the container runtime user without password prompt
    exec runuser -u opencoder -- sh -lc "cd /home/opencoder/workspace && /opt/opencode/bin/opencode web --port ${OPENCODE_PORT} --hostname ${OPENCODE_HOST}"
fi
