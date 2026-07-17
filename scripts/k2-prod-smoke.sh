#!/usr/bin/env bash
# K2 production smoke — Wave 3 prep (docs/superpowers/specs/2026-07-07-okx-x402-infra-waves.md).
#
# Verifies on the live MCP endpoint (POST /mcp):
#   - Discovery tools (search_tools, compare_tools) currently do NOT return HTTP 402 (advisory).
#   - K2 check_endpoint_health returns HTTP 402 with PAYMENT-REQUIRED + accepts[] (no wallet).
# And on the OKX.AI package endpoint (ASP #4609, listed 2026-07-17):
#   - Plain GET and unpaid tools/call on /mcp/okx both answer HTTP 402 (listing contract).
#
# Environment:
#   ONCHAINAI_MCP_URL   API origin for POST /mcp (no trailing slash).
#                       Default: RAILWAY_API_URL or https://onchainai-production.up.railway.app
#   RAILWAY_API_URL     Alias for ONCHAINAI_MCP_URL (same default chain).
#   ONCHAINAI_OKX_PACKAGE_URL
#                       OKX-registered package endpoint (default: https://www.onchain-ai.xyz/mcp/okx).
#   ONCHAINAI_K2_PROBE_SLUG
#                       Listed x402 tool slug for check_endpoint_health (default: goldrush-x402).
#   ONCHAINAI_K2_COMPARE_SLUGS
#                       Comma-separated slugs for compare_tools (default: goldrush-x402,x402).
#
# Owner-only wallet E2E (CDP facilitator settle + Probe Receipt) — not run here:
#   EVM_PRIVATE_KEY=0x... node scripts/x402-premium-e2e.mjs [slug] [api_base]
#
# Usage:
#   ./scripts/k2-prod-smoke.sh
#   ONCHAINAI_MCP_URL=https://onchainai-production.up.railway.app ./scripts/k2-prod-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/smoke-test-common.sh
source "${ROOT}/scripts/smoke-test-common.sh"

MCP_URL="${ONCHAINAI_MCP_URL:-${RAILWAY_API_URL:-https://onchainai-production.up.railway.app}}"
MCP_URL="${MCP_URL%/}"
PROBE_SLUG="${ONCHAINAI_K2_PROBE_SLUG:-goldrush-x402}"
COMPARE_SLUGS="${ONCHAINAI_K2_COMPARE_SLUGS:-goldrush-x402,x402}"

k2_fail() {
  echo "K2 SMOKE FAIL: $*" >&2
  exit 1
}

# POST /mcp tools/call — sets _MCP_HTTP_CODE; body/headers in _MCP_BODY / _MCP_HEADERS.
_MCP_HTTP_CODE=""
_MCP_BODY=""
_MCP_HEADERS=""
mcp_tools_call() {
  local tool_name="$1"
  local arguments_json="$2"
  _MCP_BODY="$(mktemp)"
  _MCP_HEADERS="$(mktemp)"
  local payload
  payload="$(printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"%s","arguments":%s}}' \
    "$tool_name" "$arguments_json")"
  _MCP_HTTP_CODE="$(curl -sS -D "$_MCP_HEADERS" -o "$_MCP_BODY" -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "${MCP_URL}/mcp")" || k2_fail "POST /mcp tools/call ${tool_name} curl failed"
}

mcp_cleanup() {
  rm -f "$_MCP_BODY" "$_MCP_HEADERS"
}

assert_discovery_free() {
  local tool_name="$1"
  local arguments_json="$2"
  mcp_tools_call "$tool_name" "$arguments_json"
  local code="$_MCP_HTTP_CODE"
  if [[ "$code" == "402" ]]; then
    echo "K2 SMOKE WARN: ${tool_name} returned HTTP 402 (discovery currently free — advisory, not hard rule)" >&2
    echo "---- response body ----" >&2
    head -40 "$_MCP_BODY" >&2
    mcp_cleanup
    return 0
  fi
  if [[ "$code" != "200" ]]; then
    echo "---- response body ----" >&2
    head -40 "$_MCP_BODY" >&2
    mcp_cleanup
    k2_fail "${tool_name} expected HTTP 200, got ${code}"
  fi
  if ! grep -q '"result"' "$_MCP_BODY"; then
    echo "---- response body ----" >&2
    head -40 "$_MCP_BODY" >&2
    mcp_cleanup
    k2_fail "${tool_name} missing JSON-RPC result"
  fi
  if grep -q 'PAYMENT-REQUIRED\|payment-required\|"x402Version"' "$_MCP_BODY"; then
    echo "K2 SMOKE WARN: ${tool_name} body looks like x402 payment gate (discovery currently free — advisory)" >&2
    mcp_cleanup
    return 0
  fi
  mcp_cleanup
}

compare_slugs_json_array() {
  local IFS=,
  local -a parts=()
  local slug
  for slug in $COMPARE_SLUGS; do
    slug="${slug//[[:space:]]/}"
    [[ -n "$slug" ]] || continue
    parts+=("\"${slug}\"")
  done
  if ((${#parts[@]} < 2)); then
    k2_fail "ONCHAINAI_K2_COMPARE_SLUGS must list at least two slugs (got: ${COMPARE_SLUGS})"
  fi
  local joined
  joined="$(IFS=,; echo "${parts[*]}")"
  printf '{"slugs":[%s]}' "$joined"
}

echo "=== K2 prod smoke: ${MCP_URL}/mcp ==="

echo "--- discovery: search_tools (402 = warn, advisory) ---"
assert_discovery_free "search_tools" '{"query":"x402","limit":1}'

echo "--- discovery: compare_tools (402 = warn, advisory) ---"
compare_args="$(compare_slugs_json_array)"
assert_discovery_free "compare_tools" "$compare_args"

echo "--- K2: check_endpoint_health (must 402 + PAYMENT-REQUIRED) ---"
mcp_tools_call "check_endpoint_health" "{\"slug\":\"${PROBE_SLUG}\"}"
health_code="$_MCP_HTTP_CODE"
payment_header=""
if [[ -f "$_MCP_HEADERS" ]]; then
  payment_header="$(grep -i '^payment-required:' "$_MCP_HEADERS" | head -1 | sed 's/^[^:]*:[[:space:]]*//' || true)"
fi

if [[ "$health_code" == "503" ]]; then
  echo "---- response body ----" >&2
  head -20 "$_MCP_BODY" >&2
  mcp_cleanup
  k2_fail "check_endpoint_health returned 503 — K2 x402 not configured (set X402_PAY_TO_ADDRESS on Railway)"
fi

if [[ "$health_code" != "402" ]]; then
  echo "---- response body ----" >&2
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  k2_fail "check_endpoint_health expected HTTP 402, got ${health_code}"
fi

if [[ -z "$payment_header" ]]; then
  echo "---- response headers ----" >&2
  head -30 "$_MCP_HEADERS" >&2
  mcp_cleanup
  k2_fail "check_endpoint_health missing PAYMENT-REQUIRED header"
fi

if ! grep -q '"accepts"' "$_MCP_BODY"; then
  echo "---- response body ----" >&2
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  k2_fail "check_endpoint_health 402 body missing accepts[] payment requirements"
fi

if ! grep -q '"x402Version"' "$_MCP_BODY"; then
  mcp_cleanup
  k2_fail "check_endpoint_health 402 body missing x402Version"
fi

mcp_cleanup

# OKX.AI listing contract (ASP #4609, listed 2026-07-17): the registered package
# endpoint must answer HTTP 402 on plain GET and on unpaid tools/call. A 200 here
# usually means the OKX gate env vars (OKX_API_KEY / OKX_SECRET_KEY /
# OKX_PASSPHRASE / OKX_PAY_TO_ADDRESS) dropped off Railway — the live listing
# then breaks silently. Same probes as the scripts/register-okx-asp.sh pre-check.
OKX_PACKAGE_URL="${ONCHAINAI_OKX_PACKAGE_URL:-https://www.onchain-ai.xyz/mcp/okx}"

echo "--- OKX package: plain GET ${OKX_PACKAGE_URL} (must 402) ---"
okx_body="$(mktemp)"
okx_code="$(curl -sS -o "$okx_body" -w "%{http_code}" -H "Accept: application/json" \
  "$OKX_PACKAGE_URL")" || { rm -f "$okx_body"; k2_fail "GET ${OKX_PACKAGE_URL} curl failed"; }
if [[ "$okx_code" != "402" ]]; then
  head -20 "$okx_body" >&2
  rm -f "$okx_body"
  k2_fail "OKX package GET expected HTTP 402, got ${okx_code} — live OKX.AI listing contract; check OKX_* env vars on Railway"
fi
if ! grep -q '"x402Version"' "$okx_body"; then
  head -20 "$okx_body" >&2
  rm -f "$okx_body"
  k2_fail "OKX package GET 402 body missing x402Version challenge"
fi
rm -f "$okx_body"

echo "--- OKX package: unpaid tools/call (must 402) ---"
okx_body="$(mktemp)"
okx_code="$(curl -sS -o "$okx_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"bridge","limit":1}}}' \
  "$OKX_PACKAGE_URL")" || { rm -f "$okx_body"; k2_fail "POST ${OKX_PACKAGE_URL} curl failed"; }
if [[ "$okx_code" != "402" ]]; then
  head -20 "$okx_body" >&2
  rm -f "$okx_body"
  k2_fail "OKX package unpaid tools/call expected HTTP 402, got ${okx_code} — check OKX_* env vars on Railway"
fi
rm -f "$okx_body"

echo "K2 SMOKE PASS ${MCP_URL} (slug=${PROBE_SLUG}) + OKX package 402 contract OK"