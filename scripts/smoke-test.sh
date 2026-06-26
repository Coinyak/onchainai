#!/usr/bin/env bash
set -euo pipefail

BASE="${1:-http://localhost:3000}"
BASE="${BASE%/}"

fail() {
  echo "SMOKE FAIL: $*" >&2
  exit 1
}

check_get() {
  local path="$1"
  local body
  body="$(mktemp)"
  code="$(curl -sS -L -o "$body" -w "%{http_code}" "${BASE}${path}")" || fail "GET ${path} curl failed"
  [[ "$code" == "200" ]] || fail "GET ${path} returned ${code}"
  if grep -qiE "error deserializing|missing field filters|panic|not found: /pkg" "$body"; then
    echo "---- body excerpt ----" >&2
    head -80 "$body" >&2
    fail "GET ${path} contains app error"
  fi
}

check_get "/"
check_get "/tools"
check_get "/tools?function=bridge&type=mcp"

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || fail "POST /mcp missing serverInfo"

echo "SMOKE PASS ${BASE}"