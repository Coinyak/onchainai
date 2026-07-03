#!/usr/bin/env bash
# Curl smoke for the Railway API-only service (Rust Axum; no Next.js pages).
#
# Usage:
#   ./scripts/smoke-test-api.sh
#   ./scripts/smoke-test-api.sh https://onchainai-production.up.railway.app
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/smoke-test-common.sh
source "${ROOT}/scripts/smoke-test-common.sh"

BASE="${1:-https://onchainai-production.up.railway.app}"
BASE="${BASE%/}"

# API hosts must not masquerade as the Next.js frontend.
probe="$(mktemp)"
code="$(curl -sS -L -o "$probe" -w "%{http_code}" "${BASE}/" 2>/dev/null || echo "000")"
if [[ "$code" == "200" ]] && grep -q '_next/static' "$probe"; then
  rm -f "$probe"
  smoke_fail "BASE looks like the Vercel frontend; pass the Railway API origin"
fi
rm -f "$probe"

for chain_svg in bitcoin bob polygon; do
  svg_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/chains/${chain_svg}.svg")" \
    || smoke_fail "GET /chains/${chain_svg}.svg curl failed"
  [[ "$svg_code" == "200" ]] || smoke_fail "GET /chains/${chain_svg}.svg returned ${svg_code}"
done

favicon_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/favicon.ico")" \
  || smoke_fail "GET /favicon.ico curl failed"
[[ "$favicon_code" == "200" ]] || smoke_fail "GET /favicon.ico returned ${favicon_code}"

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || smoke_fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || smoke_fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || smoke_fail "POST /mcp missing serverInfo"

mcp_tools_body="$(mktemp)"
mcp_tools_code="$(curl -sS -o "$mcp_tools_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  "${BASE}/mcp")" || smoke_fail "POST /mcp tools/list curl failed"
[[ "$mcp_tools_code" == "200" ]] || smoke_fail "POST /mcp tools/list returned ${mcp_tools_code}"
grep -q '"get_dashboard_snapshot"' "$mcp_tools_body" || smoke_fail "POST /mcp tools/list missing get_dashboard_snapshot"
rm -f "$mcp_body" "$mcp_tools_body"

search_body="$(mktemp)"
search_code="$(curl -sS -o "$search_body" -w "%{http_code}" \
  "${BASE}/api/v2/tools/search?query=uniswap&limit=1")" \
  || smoke_fail "GET /api/v2/tools/search curl failed"
[[ "$search_code" == "200" ]] || smoke_fail "GET /api/v2/tools/search returned ${search_code}"
grep -q '"slug"' "$search_body" || smoke_fail "GET /api/v2/tools/search missing tool payload"
rm -f "$search_body"

blueprints_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/api/v2/blueprints")" \
  || smoke_fail "GET /api/v2/blueprints curl failed"
[[ "$blueprints_code" == "401" ]] || smoke_fail "GET /api/v2/blueprints expected 401, got ${blueprints_code}"

echo "SMOKE API PASS ${BASE}"