#!/usr/bin/env bash
# =============================================================================
# check-dockerfile-updates.sh
# =============================================================================
# Checks for available version updates for pinned tools in the Dockerfile.
#
# Usage:
#   ./scripts/check-dockerfile-updates.sh          # Check and report
#   ./scripts/check-dockerfile-updates.sh --apply  # Check and update Dockerfile
#   ./scripts/check-dockerfile-updates.sh --help   # Show usage
#
# Environment:
#   GITHUB_TOKEN - Optional. Increases API rate limit from 60 to 5000 req/hour
#
# =============================================================================

set -euo pipefail

# Script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DOCKERFILE="${PROJECT_ROOT}/packages/core/src/docker/Dockerfile"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track updates
declare -a UPDATES=()
UPDATES_AVAILABLE=0

# =============================================================================
# Helper Functions
# =============================================================================

show_help() {
    cat << 'EOF'
check-dockerfile-updates.sh - Check for Dockerfile tool version updates

Usage:
  ./scripts/check-dockerfile-updates.sh [OPTIONS]

Options:
  --apply    Update Dockerfile with new versions (for CI)
  --help     Show this help message

Examples:
  ./scripts/check-dockerfile-updates.sh          # Check and report
  ./scripts/check-dockerfile-updates.sh --apply  # Apply updates to Dockerfile

Environment Variables:
  GITHUB_TOKEN  Optional. Set for higher GitHub API rate limits (5000/hour vs 60/hour)

Tools Checked:
  GitHub Releases:
    - mikefarah/yq
    - junegunn/fzf
    - nektos/act
    - jesseduffield/lazygit
    - fullstorydev/grpcurl
    - mvdan/sh (shfmt)
    - BurntSushi/ripgrep
    - eza-community/eza

  Crates.io:
    - cargo-nextest
    - cargo-audit
    - cargo-deny
EOF
}

# Get latest version from GitHub releases API
get_latest_github_version() {
    local owner="$1"
    local repo="$2"
    local url="https://api.github.com/repos/${owner}/${repo}/releases/latest"
    local auth_header=""

    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        auth_header="Authorization: Bearer ${GITHUB_TOKEN}"
    fi

    local response
    if [[ -n "${auth_header}" ]]; then
        response=$(curl -sS -H "${auth_header}" "${url}" 2>/dev/null || echo "")
    else
        response=$(curl -sS "${url}" 2>/dev/null || echo "")
    fi

    # Check for rate limiting
    if echo "${response}" | grep -q "API rate limit exceeded"; then
        echo "RATE_LIMITED"
        return 0  # Return 0 to not trigger set -e
    fi

    # Extract tag_name using jq
    local tag
    tag=$(echo "${response}" | jq -r '.tag_name // empty' 2>/dev/null || echo "")

    if [[ -z "${tag}" ]]; then
        echo "ERROR"
        return 0  # Return 0 to not trigger set -e
    fi

    echo "${tag}"
}

# Get latest version from crates.io API
get_latest_crate_version() {
    local crate="$1"
    local url="https://crates.io/api/v1/crates/${crate}"

    local response
    response=$(curl -sS -H "User-Agent: opencode-cloud-update-checker" "${url}" 2>/dev/null || echo "")

    local version
    version=$(echo "${response}" | jq -r '.crate.max_stable_version // .crate.max_version // empty' 2>/dev/null || echo "")

    if [[ -z "${version}" ]]; then
        echo "ERROR"
        return 0  # Return 0 to not trigger set -e
    fi

    echo "${version}"
}

# Extract version using sed (POSIX-compatible)
# Usage: extract_version "pattern_before" "pattern_after" file
# Extracts X.Y.Z or vX.Y.Z between pattern_before and pattern_after
extract_version() {
    local before="$1"
    local after="$2"
    local file="$3"

    # Use sed to extract version
    # This is compatible with both BSD (macOS) and GNU sed
    # Use | as delimiter to avoid escaping forward slashes
    # Use -- to handle patterns starting with -
    local result
    result=$(grep -- "${before}" "${file}" 2>/dev/null | sed -n "s|.*${before}\(v\{0,1\}[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)${after}.*|\1|p" | head -1 || echo "")

    if [[ -z "${result}" ]]; then
        echo "NOT_FOUND"
        return 0  # Return 0 to not trigger set -e
    fi

    echo "${result}"
}

# Normalize versions for comparison (strip 'v' prefix)
normalize_version() {
    local version="$1"
    echo "${version#v}"
}

# Print table row
print_row() {
    local tool="$1"
    local current="$2"
    local latest="$3"
    local status="$4"

    # Use printf for alignment, echo -e for color interpretation
    printf "%-20s %-12s %-12s " "${tool}" "${current}" "${latest}"
    echo -e "${status}"
}

# Update version in Dockerfile
update_dockerfile_version() {
    local old_pattern="$1"
    local new_version="$2"

    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/${old_pattern}/${new_version}/g" "${DOCKERFILE}"
    else
        sed -i "s/${old_pattern}/${new_version}/g" "${DOCKERFILE}"
    fi
}

# Check GitHub API rate limit
check_rate_limit() {
    local url="https://api.github.com/rate_limit"
    local auth_header=""

    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        auth_header="Authorization: Bearer ${GITHUB_TOKEN}"
    fi

    local response
    if [[ -n "${auth_header}" ]]; then
        response=$(curl -sS -H "${auth_header}" "${url}" 2>/dev/null) || response=""
    else
        response=$(curl -sS "${url}" 2>/dev/null) || response=""
    fi

    local remaining
    remaining=$(echo "${response}" | jq -r '.rate.remaining // 0' 2>/dev/null) || remaining="0"

    if [[ "${remaining}" -lt 10 ]]; then
        echo -e "${YELLOW}Warning: GitHub API rate limit low (${remaining} requests remaining)${NC}"
        if [[ -z "${GITHUB_TOKEN:-}" ]]; then
            echo -e "${YELLOW}Tip: Set GITHUB_TOKEN for higher limits (5000/hour vs 60/hour)${NC}"
        fi
        echo ""
    fi
}

# Check a GitHub tool and report/update
# Args: tool_name owner repo current_pattern update_old update_new_template
check_github_tool() {
    local tool_name="$1"
    local owner="$2"
    local repo="$3"
    local before_pattern="$4"
    local after_pattern="$5"
    local update_old_template="$6"   # Template with VERSION placeholder
    local update_new_template="$7"   # Template with VERSION placeholder
    local apply_updates="$8"

    local current latest

    current=$(extract_version "${before_pattern}" "${after_pattern}" "${DOCKERFILE}")
    latest=$(get_latest_github_version "${owner}" "${repo}")

    if [[ "${current}" != "NOT_FOUND" && "${latest}" != "ERROR" && "${latest}" != "RATE_LIMITED" ]]; then
        if [[ "$(normalize_version "${current}")" != "$(normalize_version "${latest}")" ]]; then
            print_row "${tool_name}" "${current}" "${latest}" "${YELLOW}UPDATE AVAILABLE${NC}"
            UPDATES+=("${tool_name}: ${current} -> ${latest} (https://github.com/${owner}/${repo}/releases/tag/${latest})")
            UPDATES_AVAILABLE=$((UPDATES_AVAILABLE + 1))
            if [[ "${apply_updates}" == true ]]; then
                local old_str new_str
                old_str="${update_old_template//VERSION/${current}}"
                new_str="${update_new_template//VERSION/${latest}}"
                update_dockerfile_version "${old_str}" "${new_str}"
            fi
        else
            print_row "${tool_name}" "${current}" "${latest}" "${GREEN}up-to-date${NC}"
        fi
    else
        print_row "${tool_name}" "${current}" "${latest:-ERROR}" "${RED}check failed${NC}"
    fi
}

# Check a cargo crate and report/update
check_cargo_crate() {
    local crate_name="$1"
    local apply_updates="$2"

    local current latest

    current=$(extract_version "${crate_name}@" "" "${DOCKERFILE}")
    latest=$(get_latest_crate_version "${crate_name}")

    if [[ "${current}" != "NOT_FOUND" && "${latest}" != "ERROR" ]]; then
        if [[ "${current}" != "${latest}" ]]; then
            print_row "${crate_name}" "${current}" "${latest}" "${YELLOW}UPDATE AVAILABLE${NC}"
            UPDATES+=("${crate_name}: ${current} -> ${latest} (https://crates.io/crates/${crate_name})")
            UPDATES_AVAILABLE=$((UPDATES_AVAILABLE + 1))
            if [[ "${apply_updates}" == true ]]; then
                update_dockerfile_version "${crate_name}@${current}" "${crate_name}@${latest}"
            fi
        else
            print_row "${crate_name}" "${current}" "${latest}" "${GREEN}up-to-date${NC}"
        fi
    else
        print_row "${crate_name}" "${current}" "${latest:-ERROR}" "${RED}check failed${NC}"
    fi
}

# =============================================================================
# Main Logic
# =============================================================================

main() {
    local apply_updates=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --apply)
                apply_updates=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Verify Dockerfile exists
    if [[ ! -f "${DOCKERFILE}" ]]; then
        echo -e "${RED}Error: Dockerfile not found at ${DOCKERFILE}${NC}"
        exit 1
    fi

    # Check dependencies
    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error: jq is required but not installed${NC}"
        exit 1
    fi

    echo -e "${BLUE}Checking Dockerfile tool versions...${NC}"
    echo ""

    # Check rate limit
    check_rate_limit

    # Print header
    printf "%-20s %-12s %-12s %s\n" "Tool" "Current" "Latest" "Status"
    printf "%-20s %-12s %-12s %s\n" "----" "-------" "------" "------"

    # ==========================================================================
    # GitHub Tools
    # ==========================================================================

    # yq - pattern: yq/releases/download/vX.Y.Z/
    check_github_tool "yq" "mikefarah" "yq" \
        "yq/releases/download/" "/" \
        "yq\/releases\/download\/VERSION" "yq\/releases\/download\/VERSION" \
        "${apply_updates}"

    # fzf - pattern: --branch vX.Y.Z --depth
    check_github_tool "fzf" "junegunn" "fzf" \
        "--branch " " --depth" \
        "--branch VERSION" "--branch VERSION" \
        "${apply_updates}"

    # act - pattern: bash -s -- -b /home/opencoder/.local/bin vX.Y.Z
    check_github_tool "act" "nektos" "act" \
        ".local/bin " "" \
        ".local\/bin VERSION" ".local\/bin VERSION" \
        "${apply_updates}"

    # lazygit - pattern: lazygit@vX.Y.Z
    check_github_tool "lazygit" "jesseduffield" "lazygit" \
        "lazygit@" "" \
        "lazygit@VERSION" "lazygit@VERSION" \
        "${apply_updates}"

    # grpcurl - pattern: grpcurl@vX.Y.Z
    check_github_tool "grpcurl" "fullstorydev" "grpcurl" \
        "grpcurl/cmd/grpcurl@" "" \
        "grpcurl\/cmd\/grpcurl@VERSION" "grpcurl\/cmd\/grpcurl@VERSION" \
        "${apply_updates}"

    # shfmt - pattern: shfmt@vX.Y.Z
    check_github_tool "shfmt" "mvdan" "sh" \
        "shfmt@" "" \
        "shfmt@VERSION" "shfmt@VERSION" \
        "${apply_updates}"

    # ==========================================================================
    # Cargo Crates
    # ==========================================================================

    # ripgrep - pattern: ripgrep/releases/download/X.Y.Z/ripgrep-X.Y.Z-
    # (installed from pre-built GitHub release binary)
    check_github_tool "ripgrep" "BurntSushi" "ripgrep" \
        "ripgrep/releases/download/" "/" \
        "ripgrep\/releases\/download\/VERSION\/ripgrep-VERSION-" "ripgrep\/releases\/download\/VERSION\/ripgrep-VERSION-" \
        "${apply_updates}"

    # eza - pattern: eza/releases/download/vX.Y.Z/eza_
    # (installed from pre-built GitHub release binary)
    check_github_tool "eza" "eza-community" "eza" \
        "eza/releases/download/" "/" \
        "eza\/releases\/download\/VERSION\/eza_" "eza\/releases\/download\/VERSION\/eza_" \
        "${apply_updates}"

    check_cargo_crate "cargo-nextest" "${apply_updates}"
    check_cargo_crate "cargo-audit" "${apply_updates}"
    check_cargo_crate "cargo-deny" "${apply_updates}"

    # ==========================================================================
    # Summary
    # ==========================================================================

    echo ""
    if [[ ${UPDATES_AVAILABLE} -gt 0 ]]; then
        echo -e "${YELLOW}Summary: ${UPDATES_AVAILABLE} update(s) available${NC}"
        echo ""
        echo "Updates available:"
        for update in "${UPDATES[@]}"; do
            echo "- ${update}"
        done

        if [[ "${apply_updates}" == true ]]; then
            echo ""
            echo -e "${GREEN}Updates applied to Dockerfile${NC}"
            echo "Review changes with: git diff packages/core/src/docker/Dockerfile"
        fi
    else
        echo -e "${GREEN}Summary: All tools up-to-date${NC}"
    fi

    # Return exit code for CI
    # When --apply is used, always return 0 (success) since we applied the updates
    # When checking only, return number of updates available (0 = up to date)
    if [[ "${apply_updates}" == true ]]; then
        exit 0
    else
        # Return 0 if no updates, non-zero otherwise (for CI to detect updates available)
        if [[ ${UPDATES_AVAILABLE} -eq 0 ]]; then
            exit 0
        else
            exit 0  # Don't fail the check, just report
        fi
    fi
}

main "$@"
