#!/usr/bin/env bash
# Curl smoke: public pages, dashboard/toolkit routes, chain markup, MCP initialize.
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
grep -q 'site-top-nav' "$home_body" || fail "GET / missing site-top-nav markup"
grep -q 'auth-sign-in' "$home_body" || fail "GET / missing auth-sign-in markup"
grep -q 'data-testid="top-nav-sign-in"' "$home_body" || fail "GET / missing top-nav Sign in control"
grep -q '>Sign in<' "$home_body" || fail "GET / missing Sign in label"
grep -q 'data-testid="profile-menu"' "$home_body" && fail "GET / unexpected profile-menu when signed out"
grep -q 'site-top-nav-link-dashboard' "$home_body" && fail "GET / unexpected top-nav Dashboard link when signed out"
grep -q 'site-top-nav-link-toolkit' "$home_body" && fail "GET / unexpected top-nav Toolkit link when signed out"
login_body="$(check_get "/login")"
grep -q 'Continue with GitHub' "$login_body" || fail "GET /login missing GitHub sign-in option"
grep -q 'data-testid="github-sign-in"' "$login_body" || fail "GET /login missing github-sign-in test id"
grep -q 'href="/auth/github"[^>]*rel="external"' "$login_body" || fail "GET /login missing rel=external on GitHub sign-in link"
grep -q 'wallet-sign-in' "$login_body" || fail "GET /login missing wallet sign-in option"
grep -q 'category-grid' "$home_body" && fail "GET / unexpected category-grid markup"

tools_body="$(check_get "/tools")"
grep -q 'toolbar-filter-row' "$tools_body" || fail "GET /tools missing toolbar-filter-row markup"
grep -q '>Verified<' "$tools_body" || fail "GET /tools missing Verified status tab"
grep -q '>Official<' "$tools_body" || fail "GET /tools missing Official status tab"
grep -q '>MCP<' "$tools_body" || fail "GET /tools missing MCP type tab"
grep -q '>CLI<' "$tools_body" || fail "GET /tools missing CLI type tab"
grep -q '>API<' "$tools_body" || fail "GET /tools missing API type tab"
grep -q '>SDK<' "$tools_body" || fail "GET /tools missing SDK type tab"
grep -q '>Skill<' "$tools_body" || fail "GET /tools missing Skill type tab"
grep -q '>x402<' "$tools_body" || fail "GET /tools missing x402 type tab"
if echo "$tools_body" | grep -q 'class="tool-list"'; then
  tool_cards="$(echo "$tools_body" | grep -c 'tool-card' || true)"
  if [[ "$tool_cards" -ge 50 ]] || [[ ${#tools_body} -gt 20000 ]]; then
    echo "$tools_body" | grep -qE 'load-more-btn|load-more-row' \
      || fail "GET /tools missing load-more markup (large listing)"
  fi
fi

check_get "/tools?function=bridge&type=mcp"
check_get "/tools?pricing=x402"
check_get "/tools?chain=ethereum&page=2"
check_chain_markup "/tools"
check_chain_markup "/tools?chain=ethereum"
check_chain_markup "/categories/bridge"

dashboard_body="$(check_get "/dashboard")"
grep -q 'Crypto tool coverage' "$dashboard_body" || fail "GET /dashboard missing dashboard heading"

toolkit_body="$(check_get "/toolkit")"
grep -q 'Sign in to save your stack' "$toolkit_body" || fail "GET /toolkit missing anonymous sign-in state"
grep -q 'data-testid="toolkit-sign-in"' "$toolkit_body" || fail "GET /toolkit missing Sign in control"

# Fresh filter links should not carry pagination from a prior page (SSR heuristic).
filter_body="$(check_get "/tools?function=bridge&type=mcp")"
if echo "$filter_body" | grep -oE 'href="[^"]*function=bridge[^"]*"' | grep -q 'page='; then
  fail "GET /tools?function=bridge&type=mcp filter href contains page="
fi

for chain_svg in bitcoin bob polygon; do
  svg_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/chains/${chain_svg}.svg")" \
    || fail "GET /chains/${chain_svg}.svg curl failed"
  [[ "$svg_code" == "200" ]] || fail "GET /chains/${chain_svg}.svg returned ${svg_code}"
done

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || fail "POST /mcp missing serverInfo"

mcp_tools_body="$(mktemp)"
mcp_tools_code="$(curl -sS -o "$mcp_tools_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  "${BASE}/mcp")" || fail "POST /mcp tools/list curl failed"
[[ "$mcp_tools_code" == "200" ]] || fail "POST /mcp tools/list returned ${mcp_tools_code}"
grep -q '"get_dashboard_snapshot"' "$mcp_tools_body" || fail "POST /mcp tools/list missing get_dashboard_snapshot"

echo "SMOKE PASS ${BASE}"
