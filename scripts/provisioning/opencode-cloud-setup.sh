#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# opencode-cloud setup script layout + fetch strategy
# =============================================================================
# Shared core: scripts/provisioning/opencode-cloud-setup.sh (this file)
# - Common provisioning: env loading, logging, idempotency, core install steps
# - No AWS-only dependencies (AWS data injected via env or wrapper)
#
# AWS wrappers:
# - scripts/provisioning/opencode-cloud-setup-cloudformation.sh
#   - CloudFormation signal + Secrets Manager write + AWS-specific validation
# - scripts/provisioning/opencode-cloud-setup-cloud-init.sh
#   - cloud-init status/motd handling (no AWS APIs)
#
# Fetch strategy (templates):
# - CloudFormation + cloud-init user-data install a tiny bootstrap at
#   /usr/local/bin/opencode-cloud-setup.sh that downloads these repo scripts
#   from a pinned Git ref (commit SHA or release tag), optionally verifies
#   sha256 checksums, then executes the appropriate wrapper.
# =============================================================================

echo "opencode-cloud setup: shared script stub (implemented in Task 2)" >&2
