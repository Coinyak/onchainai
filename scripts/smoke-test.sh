#!/usr/bin/env bash
# Curl smoke: public pages, chain markup, MCP initialize.
#
# Usage:
#   ./scripts/smoke-test.sh
#   ./scripts/smoke-test.sh http://localhost:3000
#   ./scripts/smoke-test.sh https://www.onchain-ai.xyz
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
  echo "$body"
}

check_chain_markup() {
  local path="$1"
  local body
  body="$(check_get "$path")"
  grep -q 'chain-strip' "$body" || fail "GET ${path} missing chain-strip markup"
  grep -q '/chains/' "$body" || fail "GET ${path} missing /chains/ logo paths"
}

home_body="$(check_get "/")"
grep -q 'sidebar-brand' "$home_body" || fail "GET / missing sidebar-brand markup"
grep -q 'category-grid' "$home_body" && fail "GET / unexpected category-grid markup"

tools_body="$(check_get "/tools")"
if echo "$tools_body" | grep -q 'class="tool-list"'; then
  tool_cards="$(echo "$tools_body" | grep -c 'tool-card' || true)"
  if [[ "$tool_cards" -ge 50 ]] || [[ ${#tools_body} -gt 20000 ]]; then
    echo "$tools_body" | grep -qE 'load-more-btn|load-more-row' \
      || fail "GET /tools missing load-more markup (large listing)"
  fi
fi

check_get "/tools?function=bridge&type=mcp"
check_get "/tools?chain=ethereum&page=2"
check_chain_markup "/tools"
check_chain_markup "/tools?chain=ethereum"
check_chain_markup "/categories/bridge"

# Fresh filter links should not carry pagination from a prior page (SSR heuristic).
filter_body="$(check_get "/tools?function=bridge&type=mcp")"
if echo "$filter_body" | grep -oE 'href="[^"]*function=bridge[^"]*"' | grep -q 'page='; then
  fail "GET /tools?function=bridge&type=mcp filter href contains page="
fi

svg_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/chains/bitcoin.svg")" \
  || fail "GET /chains/bitcoin.svg curl failed"
[[ "$svg_code" == "200" ]] || fail "GET /chains/bitcoin.svg returned ${svg_code}"

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || fail "POST /mcp missing serverInfo"

echo "SMOKE PASS ${BASE}"