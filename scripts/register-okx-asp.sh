#!/usr/bin/env bash
# OKX AI Agent Marketplace — A2MCP ASP register + activate (W6).
#
# Prerequisites:
#   1. onchainos CLI: curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
#   2. Agentic Wallet login:
#        onchainos wallet login <email>
#        onchainos wallet verify <OTP>
#
# Default: paid SKUs only (4 A2MCP services on POST /mcp).
# Discovery tools stay free on the MCP endpoint but are not OKX-listed.
#
# Usage:
#   ./scripts/register-okx-asp.sh
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
LANG="${OKX_ASP_LANG:-en-US}"

echo "== pre-check (asp) =="
precheck="$(onchainos agent pre-check --role asp)"
echo "${precheck}"
can_create="$(echo "${precheck}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{}).get('canCreate', d.get('canCreate', False)))" 2>/dev/null || echo false)"
if [[ "${can_create}" != "True" && "${can_create}" != "true" ]]; then
  consent_key="$(echo "${precheck}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{}).get('consent',{}).get('consentKey',''))" 2>/dev/null || true)"
  if [[ -n "${consent_key}" ]]; then
    echo "== accept marketplace terms (first-time wallet) =="
    precheck="$(onchainos agent pre-check --role asp --consent-key "${consent_key}")"
    echo "${precheck}"
    can_create="$(echo "${precheck}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{}).get('canCreate', d.get('canCreate', False)))" 2>/dev/null || echo false)"
  fi
fi
if [[ "${can_create}" != "True" && "${can_create}" != "true" ]]; then
  echo "Cannot create ASP under this wallet (see pre-check above). Use update/activate on existing ASP or switch wallet." >&2
  exit 1
fi

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

SERVICE_JSON='[
  {"serviceName":"Trust Probe Endpoint Health","serviceDescription":"On-demand x402 endpoint liveness check before calling third-party paid APIs.\n1. Tool slug from search_tools results","serviceType":"A2MCP","fee":"0.003","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Agent Toolkit Export","serviceDescription":"Export approved crypto tools as JSON and markdown install kit for AI agents.\n1. Tool slugs (up to 25) or a function category id","serviceType":"A2MCP","fee":"0.01","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Verified Tool Recommendation","serviceDescription":"Returns a single verified live x402 tool for a task intent. Probes top candidates on-demand for liveness and price honesty.\n1. Natural-language intent (required)\n2. Optional chain or function filter","serviceType":"A2MCP","fee":"0.01","endpoint":"'"${ENDPOINT}"'"},
  {"serviceName":"Agent Gap Audit","serviceDescription":"Decomposes a task intent into subgoals and maps each to OnchainAI catalog tools, surfacing gaps where no tools exist.\n1. Natural-language task intent (required)","serviceType":"A2MCP","fee":"0.05","endpoint":"'"${ENDPOINT}"'"}
]'

echo "== validate-listing =="
validate_out="$(onchainos agent validate-listing --role asp --name "${NAME}" --description "${DESCRIPTION}" --service "${SERVICE_JSON}")"
echo "${validate_out}"
pass="$(echo "${validate_out}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass', False))" 2>/dev/null || echo false)"
if [[ "${pass}" != "True" && "${pass}" != "true" ]]; then
  echo "validate-listing failed — fix findings before create." >&2
  exit 1
fi

echo "== create ASP =="
create_out="$(onchainos agent create \
  --role asp \
  --name "${NAME}" \
  --description "${DESCRIPTION}" \
  --picture "${picture_url}" \
  --service "${SERVICE_JSON}")"
echo "${create_out}"

agent_id="$(echo "${create_out}" | python3 -c "
import sys, json
d = json.load(sys.stdin)
data = d.get('data', d)
for key in ('newAgentId', 'agentId', 'id'):
    v = data.get(key)
    if v:
        print(v)
        break
" 2>/dev/null || true)"

if [[ -z "${agent_id}" || "${agent_id}" == "None" ]]; then
  echo "create succeeded but agent id missing — run: onchainos agent get-my-agents --role asp" >&2
  agents="$(onchainos agent get-my-agents --role asp)"
  echo "${agents}"
  agent_id="$(echo "${agents}" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('data', d)
if isinstance(items, dict):
    items = items.get('list', items.get('agents', []))
if not items:
    sys.exit(0)
last = items[-1] if isinstance(items, list) else items
print(last.get('agentId', last.get('id', '')))
" 2>/dev/null || true)"
fi

if [[ -z "${agent_id}" ]]; then
  echo "Could not resolve agent id for activate." >&2
  exit 1
fi

echo "== activate #${agent_id} (review ~2 business days) =="
activate_out="$(onchainos agent activate --agent-id "${agent_id}" --preferred-language "${LANG}")"
echo "${activate_out}"

echo ""
echo "Done. ASP #${agent_id} submitted for OKX review."
echo "Check okx.ai/agents and Agentic Wallet email for approval status."