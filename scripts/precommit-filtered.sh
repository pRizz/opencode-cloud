#!/usr/bin/env bash
# Wrapper for 'just pre-commit' that only outputs failures to reduce context bloat

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

LOG_FILE="/tmp/opencode-cloud-precommit-$$.log"

echo "Running pre-commit checks (full output captured to $LOG_FILE)..."
echo ""

# Run pre-commit, capture all output
if just pre-commit > "$LOG_FILE" 2>&1; then
    echo "✅ All tests passed."
    rm -f "$LOG_FILE"
    exit 0
else
    EXIT_CODE=$?
    echo "❌ Pre-commit failed (exit code $EXIT_CODE)"
    echo ""
    echo "=== FAILURES ONLY ==="
    
    # Extract only failure lines and surrounding context
    grep -B2 -A2 "FAILED\|❌\|error:\|Error:" "$LOG_FILE" || true
    
    # Show summary line if present
    grep "Tests:" "$LOG_FILE" || true
    
    echo ""
    echo "Full log saved: $LOG_FILE"
    exit $EXIT_CODE
fi
