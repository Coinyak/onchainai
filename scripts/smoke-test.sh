#!/usr/bin/env bash
# Curl smoke for local Next.js UI + proxied Rust API (CI + restart-dev).
#
# Split production deploy:
#   ./scripts/smoke-test-api.sh      — Railway API-only service
#   ./scripts/smoke-test-frontend.sh — Vercel Next.js frontend
#
# Usage:
#   ./scripts/smoke-test.sh
#   ./scripts/smoke-test.sh http://localhost:3000
set -euo pipefail

BASE="${1:-http://localhost:3000}"
BASE="${BASE%/}"
# Local split stack (Next :3000 + API :3001): set ONCHAINAI_SMOKE_API_BASE for MCP POST.
API_BASE="${ONCHAINAI_SMOKE_API_BASE:-$BASE}"
API_BASE="${API_BASE%/}"

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
grep -q '_next/static' "$home_body" || fail "GET / missing Next.js bundle markers"
grep -q 'category-grid' "$home_body" && fail "GET / unexpected category-grid markup"

login_body="$(check_get "/login")"
grep -qE 'Continue with GitHub|Loading sign-in' "$login_body" \
  || fail "GET /login missing sign-in shell (Next.js client page)"
grep -q '_next/static' "$login_body" || fail "GET /login missing Next.js bundle markers"

tools_body="$(check_get "/tools")"
grep -q 'tool-card' "$tools_body" || fail "GET /tools missing tool-card markup"
grep -q '_next/static' "$tools_body" || fail "GET /tools missing Next.js bundle markers"

check_get "/tools?function=bridge&type=mcp"
check_get "/tools?pricing=x402"
check_get "/tools?chain=ethereum&page=2"
if grep -q 'chain-strip' "$tools_body"; then
  check_chain_markup "/tools?chain=ethereum"
fi
check_get "/categories/bridge" >/dev/null

dashboard_body="$(check_get "/dashboard")"
grep -qE 'admin_required|Loading sign-in|Sign in to OnchainAI' "$dashboard_body" \
  || fail "GET /dashboard should gate anonymous users to login"

toolkit_body="$(check_get "/toolkit")"
grep -q 'Sign in to save' "$toolkit_body" || fail "GET /toolkit missing anonymous sign-in state"
grep -q 'data-testid="toolkit-sign-in"' "$toolkit_body" || fail "GET /toolkit missing Sign in control"
connect_body="$(check_get "/connect")"
grep -q '_next/static' "$connect_body" || fail "GET /connect missing Next.js bundle markers"

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
  "${API_BASE}/mcp")" || fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || fail "POST /mcp missing serverInfo"

mcp_tools_body="$(mktemp)"
mcp_tools_code="$(curl -sS -o "$mcp_tools_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  "${API_BASE}/mcp")" || fail "POST /mcp tools/list curl failed"
[[ "$mcp_tools_code" == "200" ]] || fail "POST /mcp tools/list returned ${mcp_tools_code}"
grep -q '"get_dashboard_snapshot"' "$mcp_tools_body" || fail "POST /mcp tools/list missing get_dashboard_snapshot"

echo "SMOKE PASS ${BASE}"
