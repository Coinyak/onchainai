#!/usr/bin/env bash
# OKX AI Agent Marketplace — A2MCP ASP register or re-submit (W6).
#
# Prerequisites:
#   1. onchainos CLI: curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
#   2. Agentic Wallet login:
#        onchainos wallet login <email>
#        onchainos wallet verify <OTP>
#
# Default: 12 A2MCP services on POST /mcp at $0.1 USDT/call (OKX Broker, X Layer).
# Re-submit existing ASP #4609 via update + activate.
#
# Usage:
#   ./scripts/register-okx-asp.sh
#   OKX_ASP_AGENT_ID=4609 ./scripts/register-okx-asp.sh
#   OKX_ASP_LANG=ko-KR ./scripts/register-okx-asp.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${HOME}/.local/bin:${PATH}"

if ! command -v onchainos >/dev/null 2>&1; then
  echo "onchainos CLI not found. Install: curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh" >&2
  exit 1
fi

logged_in="$(onchainos wallet status 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{}).get('loggedIn', False))" 2>/dev/null || echo false)"
if [[ "${logged_in}" != "True" && "${logged_in}" != "true" ]]; then
  echo "Agentic Wallet not logged in. Run:" >&2
  echo "  onchainos wallet login <email>" >&2
  echo "  onchainos wallet verify <OTP>" >&2
  exit 1
fi

AVATAR="${ROOT}/public/brand/onchainai-icon-512.png"
if [[ ! -f "${AVATAR}" ]]; then
  echo "Avatar not found: ${AVATAR}" >&2
  exit 1
fi

NAME="OnchainAI"
DESCRIPTION="Crypto tool directory for AI agents — discover, compare, and vet MCP, CLI, SDK, API, and x402 tools with trust metadata."
ENDPOINT="https://www.onchain-ai.xyz/mcp"
FEE="0.1"
LANG="${OKX_ASP_LANG:-en-US}"
AGENT_ID="${OKX_ASP_AGENT_ID:-4609}"

# Full listing for validate-listing (all 12 SKUs, single price).
VALIDATE_SERVICE_JSON='[
  {"serviceName":"Crypto Tool Search","serviceDescription":"Search the OnchainAI catalog by keyword, chain, category, or x402 filter.\n1. Query string (required)\n2. Optional chain, category, sort, limit","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Tool Detail Lookup","serviceDescription":"Full tool profile with trust signals, install risk, chains, and x402 pricing metadata.\n1. Tool slug (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Install Guide","serviceDescription":"Platform-specific install steps and risk assessment for a catalog tool.\n1. Tool slug (required)\n2. Platform: claude, cursor, generic, or cli","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Category Listing","serviceDescription":"List all tool categories with live counts from the OnchainAI directory.\n1. No input required","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Catalog Dashboard Snapshot","serviceDescription":"Public catalog overview: tool counts, x402 stats, featured tools, and chain coverage.\n1. Optional limit (max 12)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Tool Comparison","serviceDescription":"Side-by-side comparison of 2–4 tools: trust, chains, pricing, install risk, and x402 metadata.\n1. Tool slugs array (2–4 required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"x402 Price History","serviceDescription":"Historical x402 pricing for a listed tool from probe data.\n1. Tool slug (required)\n2. Optional days window","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"x402 Market Trends","serviceDescription":"x402 ecosystem trends: pricing shifts, network distribution, and catalog stats.\n1. Optional days window","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Trust Probe Endpoint Health","serviceDescription":"On-demand x402 endpoint liveness check before calling third-party paid APIs.\n1. Tool slug from search results (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Agent Toolkit Export","serviceDescription":"Export approved crypto tools as JSON and markdown install kit for AI agents.\n1. Tool slugs (up to 25) or a function category id","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Verified Tool Recommendation","serviceDescription":"Returns a single verified live x402 tool for a task intent. Probes top candidates for liveness and price honesty.\n1. Natural-language intent (required)\n2. Optional chain or function filter","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Agent Gap Audit","serviceDescription":"Decomposes a task intent into subgoals and maps each to OnchainAI catalog tools, surfacing gaps where no tools exist.\n1. Natural-language task intent (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"}
]'

# Incremental update for ASP #4609: bump 4 existing SKUs + add 8 discovery SKUs.
UPDATE_SERVICE_JSON='[
  {"operation":"update","id":"27814","serviceName":"Trust Probe Endpoint Health","serviceDescription":"On-demand x402 endpoint liveness check before calling third-party paid APIs.\n1. Tool slug from search results (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"update","id":"27815","serviceName":"Agent Toolkit Export","serviceDescription":"Export approved crypto tools as JSON and markdown install kit for AI agents.\n1. Tool slugs (up to 25) or a function category id","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"update","id":"27816","serviceName":"Verified Tool Recommendation","serviceDescription":"Returns a single verified live x402 tool for a task intent. Probes top candidates for liveness and price honesty.\n1. Natural-language intent (required)\n2. Optional chain or function filter","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"update","id":"27817","serviceName":"Agent Gap Audit","serviceDescription":"Decomposes a task intent into subgoals and maps each to OnchainAI catalog tools, surfacing gaps where no tools exist.\n1. Natural-language task intent (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Crypto Tool Search","serviceDescription":"Search the OnchainAI catalog by keyword, chain, category, or x402 filter.\n1. Query string (required)\n2. Optional chain, category, sort, limit","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Tool Detail Lookup","serviceDescription":"Full tool profile with trust signals, install risk, chains, and x402 pricing metadata.\n1. Tool slug (required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Install Guide","serviceDescription":"Platform-specific install steps and risk assessment for a catalog tool.\n1. Tool slug (required)\n2. Platform: claude, cursor, generic, or cli","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Category Listing","serviceDescription":"List all tool categories with live counts from the OnchainAI directory.\n1. No input required","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Catalog Dashboard Snapshot","serviceDescription":"Public catalog overview: tool counts, x402 stats, featured tools, and chain coverage.\n1. Optional limit (max 12)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"Tool Comparison","serviceDescription":"Side-by-side comparison of 2–4 tools: trust, chains, pricing, install risk, and x402 metadata.\n1. Tool slugs array (2–4 required)","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"x402 Price History","serviceDescription":"Historical x402 pricing for a listed tool from probe data.\n1. Tool slug (required)\n2. Optional days window","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"},
  {"operation":"create","serviceName":"x402 Market Trends","serviceDescription":"x402 ecosystem trends: pricing shifts, network distribution, and catalog stats.\n1. Optional days window","serviceType":"A2MCP","fee":"'"${FEE}"'","endpoint":"'"${ENDPOINT}"'"}
]'

echo "== pre-check (asp) =="
precheck="$(onchainos agent pre-check --role asp)"
echo "${precheck}"

echo "== x402 endpoint check =="
x402_check="$(onchainos agent x402-check --endpoint "${ENDPOINT}" 2>/dev/null || true)"
echo "${x402_check}"

echo "== upload avatar =="
upload_out="$(onchainos agent upload --file "${AVATAR}")"
echo "${upload_out}"
picture_url="$(echo "${upload_out}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{}).get('url', d.get('data',{}).get('pictureUrl','')))" 2>/dev/null || true)"
if [[ -z "${picture_url}" ]]; then
  picture_url="$(echo "${upload_out}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('url',''))" 2>/dev/null || true)"
fi
if [[ -z "${picture_url}" ]]; then
  echo "Avatar upload did not return a URL." >&2
  exit 1
fi

echo "== validate-listing (12 SKUs @ ${FEE} USDT) =="
validate_out="$(onchainos agent validate-listing --role asp --name "${NAME}" --description "${DESCRIPTION}" --service "${VALIDATE_SERVICE_JSON}")"
echo "${validate_out}"
pass="$(echo "${validate_out}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass', False))" 2>/dev/null || echo false)"
if [[ "${pass}" != "True" && "${pass}" != "true" ]]; then
  echo "validate-listing failed — fix findings before submit." >&2
  exit 1
fi

if [[ -n "${AGENT_ID}" ]]; then
  echo "== update ASP #${AGENT_ID} =="
  update_out="$(onchainos agent update \
    --agent-id "${AGENT_ID}" \
    --name "${NAME}" \
    --description "${DESCRIPTION}" \
    --picture "${picture_url}" \
    --service "${UPDATE_SERVICE_JSON}")"
  echo "${update_out}"
else
  echo "== create ASP =="
  create_out="$(onchainos agent create \
    --role asp \
    --name "${NAME}" \
    --description "${DESCRIPTION}" \
    --picture "${picture_url}" \
    --service "${VALIDATE_SERVICE_JSON}")"
  echo "${create_out}"
  AGENT_ID="$(echo "${create_out}" | python3 -c "
import sys, json
d = json.load(sys.stdin)
data = d.get('data', d)
for key in ('newAgentId', 'agentId', 'id'):
    v = data.get(key)
    if v:
        print(v)
        break
" 2>/dev/null || true)"
fi

if [[ -z "${AGENT_ID}" || "${AGENT_ID}" == "None" ]]; then
  echo "Could not resolve agent id for activate." >&2
  exit 1
fi

echo "== activate #${AGENT_ID} (review ~2 business days) =="
activate_out="$(onchainos agent activate --agent-id "${AGENT_ID}" --preferred-language "${LANG}")"
echo "${activate_out}"

echo ""
echo "Done. ASP #${AGENT_ID} submitted for OKX review (12 A2MCP SKUs @ ${FEE} USDT/call)."
echo "Check okx.ai/agents and Agentic Wallet email for approval status."