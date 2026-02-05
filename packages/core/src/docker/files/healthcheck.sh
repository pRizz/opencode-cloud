#!/bin/sh
set -eu

opencode_port="${OPENCODE_PORT:-${PORT:-3000}}"

if [ -d /run/systemd/system ]; then
    systemctl is-active --quiet opencode-broker.service
else
    pgrep -x opencode-broker >/dev/null
fi

test -S /run/opencode/auth.sock
curl -fsS -H "Accept: text/html" "http://localhost:${opencode_port}/" >/dev/null
