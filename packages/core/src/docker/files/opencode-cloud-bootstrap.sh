#!/bin/bash
set -euo pipefail

STATE_DIR="/var/lib/opencode-users"
STATE_FILE="${STATE_DIR}/.initial-otp.json"
SECRET_FILE="${STATE_DIR}/.initial-otp.secret"
LOCK_FILE="${STATE_DIR}/.initial-otp.lock"
COMPLETE_MARKER_FILE="${STATE_DIR}/.bootstrap-complete.json"
PROTECTED_USER="opencoder"
BUILTIN_USERS_FILE="/etc/opencode-cloud/builtin-home-users.txt"
FALLBACK_BUILTIN_HOME_USERS=("opencoder" "ubuntu")
BUILTIN_HOME_USERS=()

json_ok() {
    jq -cn "$@"
}

json_error() {
    local code="$1"
    local message="$2"
    local status="$3"
    jq -cn --arg code "${code}" --arg message "${message}" --argjson status "${status}" \
        '{ok:false,code:$code,message:$message,status:$status}'
}

ensure_state_dir() {
    install -d -m 0700 "${STATE_DIR}"
    load_builtin_home_users
}

acquire_lock() {
    exec 9>"${LOCK_FILE}"
    flock -x 9
}

generate_random_hex() {
    local byte_count="$1"
    od -An -N "${byte_count}" -tx1 /dev/urandom | tr -d ' \n'
}

utc_now() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

hash_salted_otp() {
    local salt="$1"
    local otp="$2"
    printf "%s" "${salt}:${otp}" | sha256sum | awk '{print $1}'
}

state_is_active() {
    [ -f "${STATE_FILE}" ] && [ -f "${SECRET_FILE}" ]
}

remove_state_files() {
    rm -f "${STATE_FILE}" "${SECRET_FILE}"
}

bootstrap_is_completed() {
    [ -f "${COMPLETE_MARKER_FILE}" ]
}

mark_bootstrap_completed() {
    local completed_at
    completed_at="$(utc_now)"
    umask 077
    jq -cn --arg completed_at "${completed_at}" --arg username "${PROTECTED_USER}" \
        '{completed:true,completed_at:$completed_at,username:$username}' > "${COMPLETE_MARKER_FILE}"
    chmod 600 "${COMPLETE_MARKER_FILE}"
}

get_completed_at() {
    jq -r '.completed_at // empty' "${COMPLETE_MARKER_FILE}" 2>/dev/null || true
}

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

has_non_protected_user_record() {
    # Managed user records are the canonical source of configured users.
    # Ignore image-default users so base-image accounts do not disable IOTP.
    shopt -s nullglob
    local record username
    for record in "${STATE_DIR}"/*.json; do
        username="$(jq -r '.username // empty' "${record}" 2>/dev/null || true)"
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

has_non_protected_system_user() {
    # Secondary safety net for unmanaged runtime-created users with /home entries.
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
        return 0
    done < <(getent passwd)

    return 1
}

has_non_protected_configured_user() {
    has_non_protected_user_record || has_non_protected_system_user
}

read_input_json() {
    cat
}

read_input_field() {
    local payload="$1"
    local key="$2"
    jq -r --arg key "${key}" '.[$key] // empty' <<< "${payload}" 2>/dev/null || true
}

validate_username() {
    local username="$1"
    if [[ ! "${username}" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]]; then
        return 1
    fi
    if [ "${username}" = "${PROTECTED_USER}" ]; then
        return 1
    fi
    return 0
}

password_meets_policy() {
    local password="$1"
    if [ "${#password}" -lt 12 ]; then
        return 1
    fi

    local classes=0
    [[ "${password}" =~ [[:upper:]] ]] && classes=$((classes + 1))
    [[ "${password}" =~ [[:lower:]] ]] && classes=$((classes + 1))
    [[ "${password}" =~ [[:digit:]] ]] && classes=$((classes + 1))
    [[ "${password}" =~ [^[:alnum:]] ]] && classes=$((classes + 1))

    [ "${classes}" -ge 3 ]
}

verify_ubuntu_platform() {
    local distro=""
    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        distro="$(. /etc/os-release && printf "%s" "${ID:-}")"
    fi
    [ "${distro}" = "ubuntu" ]
}

emit_inactive_and_cleanup() {
    remove_state_files
    json_ok '{ok:true,active:false,reason:"user_exists"}'
}

emit_status() {
    local include_secret="$1"
    if has_non_protected_configured_user; then
        emit_inactive_and_cleanup
        return 0
    fi

    if bootstrap_is_completed; then
        local completed_at
        completed_at="$(get_completed_at)"
        remove_state_files
        if [ -n "${completed_at}" ]; then
            jq -cn --arg completed_at "${completed_at}" \
                '{ok:true,active:false,reason:"completed",completed_at:$completed_at}'
        else
            json_ok '{ok:true,active:false,reason:"completed"}'
        fi
        return 0
    fi

    if ! state_is_active; then
        json_ok '{ok:true,active:false,reason:"not_initialized"}'
        return 0
    fi

    local created_at otp
    created_at="$(jq -r '.created_at // empty' "${STATE_FILE}" 2>/dev/null || true)"
    if [ -z "${created_at}" ]; then
        remove_state_files
        json_ok '{ok:true,active:false,reason:"invalid_state"}'
        return 0
    fi

    if [ "${include_secret}" = "1" ]; then
        otp="$(tr -d '\r\n' < "${SECRET_FILE}" 2>/dev/null || true)"
        if [ -z "${otp}" ]; then
            remove_state_files
            json_ok '{ok:true,active:false,reason:"invalid_secret"}'
            return 0
        fi
        jq -cn --arg created_at "${created_at}" --arg otp "${otp}" \
            '{ok:true,active:true,created_at:$created_at,otp:$otp}'
        return 0
    fi

    jq -cn --arg created_at "${created_at}" '{ok:true,active:true,created_at:$created_at}'
}

cmd_init_internal() {
    if has_non_protected_configured_user; then
        emit_inactive_and_cleanup
        return 0
    fi

    if bootstrap_is_completed; then
        emit_status "0"
        return 0
    fi

    if state_is_active; then
        emit_status "1"
        return 0
    fi

    remove_state_files

    local otp salt created_at otp_hash
    otp="$(generate_random_hex 48)"
    salt="$(generate_random_hex 16)"
    created_at="$(utc_now)"
    otp_hash="$(hash_salted_otp "${salt}" "${otp}")"

    jq -cn \
        --arg created_at "${created_at}" \
        --arg salt "${salt}" \
        --arg otp_hash "${otp_hash}" \
        '{version:1,active:true,created_at:$created_at,salt:$salt,otp_hash:$otp_hash}' > "${STATE_FILE}"

    umask 077
    printf "%s" "${otp}" > "${SECRET_FILE}"
    chmod 600 "${STATE_FILE}" "${SECRET_FILE}"

    jq -cn --arg created_at "${created_at}" --arg otp "${otp}" '{ok:true,active:true,created_at:$created_at,otp:$otp}'
}

cmd_init() {
    ensure_state_dir
    acquire_lock
    cmd_init_internal
}

cmd_reset() {
    ensure_state_dir
    acquire_lock

    rm -f "${COMPLETE_MARKER_FILE}"
    remove_state_files

    cmd_init_internal
}

cmd_status() {
    local include_secret="0"
    if [ "${1:-}" = "--include-secret" ]; then
        include_secret="1"
    fi

    ensure_state_dir
    acquire_lock
    emit_status "${include_secret}"
}

verify_otp_internal() {
    local otp="$1"
    local salt expected_hash actual_hash

    if ! state_is_active; then
        json_error "inactive" "Bootstrap one-time password is not active." 403
        return 0
    fi

    salt="$(jq -r '.salt // empty' "${STATE_FILE}" 2>/dev/null || true)"
    expected_hash="$(jq -r '.otp_hash // empty' "${STATE_FILE}" 2>/dev/null || true)"
    if [ -z "${salt}" ] || [ -z "${expected_hash}" ]; then
        remove_state_files
        json_error "inactive" "Bootstrap state is invalid." 403
        return 0
    fi

    actual_hash="$(hash_salted_otp "${salt}" "${otp}")"
    if [ "${actual_hash}" != "${expected_hash}" ]; then
        json_error "otp_invalid" "Initial one-time password is invalid." 401
        return 0
    fi

    json_ok '{ok:true,active:true}'
}

cmd_verify() {
    local payload otp
    payload="$(read_input_json)"
    otp="$(read_input_field "${payload}" "otp")"

    if [ -z "${otp}" ]; then
        json_error "invalid_request" "Missing one-time password." 400
        return 0
    fi

    ensure_state_dir
    acquire_lock

    if has_non_protected_configured_user; then
        emit_inactive_and_cleanup
        return 0
    fi

    verify_otp_internal "${otp}"
}

cmd_complete() {
    local payload otp verify_json verify_code
    payload="$(read_input_json)"
    otp="$(read_input_field "${payload}" "otp")"

    if [ -z "${otp}" ]; then
        json_error "invalid_request" "Missing one-time password." 400
        return 0
    fi

    ensure_state_dir
    acquire_lock

    if has_non_protected_configured_user; then
        emit_inactive_and_cleanup
        return 0
    fi

    verify_json="$(verify_otp_internal "${otp}")"
    verify_code="$(jq -r '.code // empty' <<< "${verify_json}" 2>/dev/null || true)"
    if [ -n "${verify_code}" ]; then
        printf "%s\n" "${verify_json}"
        return 0
    fi

    remove_state_files
    mark_bootstrap_completed

    jq -cn --arg username "${PROTECTED_USER}" '{ok:true,completed:true,active:false,username:$username}'
}

cmd_create_user() {
    local payload otp username password
    payload="$(read_input_json)"
    otp="$(read_input_field "${payload}" "otp")"
    username="$(read_input_field "${payload}" "username")"
    password="$(read_input_field "${payload}" "password")"

    if [ -z "${otp}" ] || [ -z "${username}" ] || [ -z "${password}" ]; then
        json_error "invalid_request" "otp, username, and password are required." 400
        return 0
    fi

    ensure_state_dir
    acquire_lock

    if has_non_protected_configured_user; then
        emit_inactive_and_cleanup
        return 0
    fi

    local verify_json verify_code
    verify_json="$(verify_otp_internal "${otp}")"
    verify_code="$(jq -r '.code // empty' <<< "${verify_json}" 2>/dev/null || true)"
    if [ -n "${verify_code}" ]; then
        printf "%s\n" "${verify_json}"
        return 0
    fi

    if ! verify_ubuntu_platform; then
        json_error "unsupported_platform" "Initial signup is currently supported only on Ubuntu containers." 400
        return 0
    fi

    if ! validate_username "${username}"; then
        json_error "invalid_username" "Username must match ^[a-z_][a-z0-9_-]{0,31}$ and cannot be reserved." 400
        return 0
    fi

    if ! password_meets_policy "${password}"; then
        json_error "invalid_password" "Password must be at least 12 characters and include 3 of 4 classes." 400
        return 0
    fi

    if id -u "${username}" >/dev/null 2>&1; then
        json_error "username_exists" "Username already exists." 409
        return 0
    fi

    if ! useradd -m -s /bin/bash "${username}" >/dev/null 2>&1; then
        json_error "create_failed" "Failed to create the Linux user account." 500
        return 0
    fi

    if ! printf "%s:%s\n" "${username}" "${password}" | chpasswd >/dev/null 2>&1; then
        userdel -r "${username}" >/dev/null 2>&1 || true
        json_error "create_failed" "Failed to set password for the new user." 500
        return 0
    fi

    local shadow_hash status locked record_path
    shadow_hash="$(getent shadow "${username}" | cut -d: -f2)"
    if [ -z "${shadow_hash}" ]; then
        json_error "create_failed" "Failed to read password hash for the new user." 500
        return 0
    fi

    status="$(passwd -S "${username}" | tr -s ' ' | cut -d' ' -f2)"
    locked="false"
    if [ "${status}" = "L" ]; then
        locked="true"
    fi

    record_path="${STATE_DIR}/${username}.json"
    umask 077
    jq -cn --arg username "${username}" --arg hash "${shadow_hash}" --argjson locked "${locked}" \
        '{username:$username,password_hash:$hash,locked:$locked}' > "${record_path}"
    chmod 600 "${record_path}"

    remove_state_files
    mark_bootstrap_completed

    jq -cn --arg username "${username}" '{ok:true,created:true,username:$username}'
}

usage() {
    cat <<'EOF'
Usage: opencode-cloud-bootstrap <command>

Commands:
  init                 Initialize bootstrap OTP if needed
  reset                Reset bootstrap completion and reinitialize OTP
  status [--include-secret]
                       Show bootstrap status
  verify               Verify OTP (expects JSON stdin: {"otp":"..."})
  complete             Complete bootstrap after verified OTP (expects JSON stdin: {"otp":"..."})
  create-user          Create first user (expects JSON stdin: {"otp":"...","username":"...","password":"..."})
EOF
}

main() {
    local command="${1:-}"
    shift || true

    case "${command}" in
        init)
            cmd_init "$@"
            ;;
        reset)
            cmd_reset "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        verify)
            cmd_verify "$@"
            ;;
        complete)
            cmd_complete "$@"
            ;;
        create-user)
            cmd_create_user "$@"
            ;;
        *)
            usage >&2
            exit 1
            ;;
    esac
}

main "$@"
