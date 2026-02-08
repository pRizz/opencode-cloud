#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <crate-name> <version>" >&2
  exit 2
fi

crate_name="$1"
crate_version="$2"
api_url="https://crates.io/api/v1/crates/${crate_name}/${crate_version}"

tmp_response="$(mktemp)"
trap 'rm -f "${tmp_response}"' EXIT

http_status="$(
  curl \
    --silent \
    --show-error \
    --location \
    --user-agent "opencode-cloud-ci (https://github.com/pRizz/opencode-container)" \
    --output "${tmp_response}" \
    --write-out '%{http_code}' \
    "${api_url}"
)"

case "${http_status}" in
  200)
    parsed_version="$(
      python3 - "${tmp_response}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text())
print(data.get("version", {}).get("num", ""))
PY
    )"
    if [ "${parsed_version}" != "${crate_version}" ]; then
      echo "Error: crates.io returned unexpected version for ${crate_name}." >&2
      echo "Expected: ${crate_version}" >&2
      echo "Actual:   ${parsed_version:-<empty>}" >&2
      exit 2
    fi
    echo "Published on crates.io: ${crate_name} ${crate_version}"
    exit 0
    ;;
  404)
    echo "Not yet published on crates.io: ${crate_name} ${crate_version}"
    exit 1
    ;;
  *)
    echo "Error: Unexpected crates.io response (${http_status}) for ${crate_name} ${crate_version}" >&2
    echo "URL: ${api_url}" >&2
    echo "Body (first 400 chars):" >&2
    head -c 400 "${tmp_response}" >&2 || true
    echo >&2
    exit 2
    ;;
esac
