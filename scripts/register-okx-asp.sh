#!/usr/bin/env bash
# OKX AI Agent Marketplace — single bundled A2MCP ASP register / re-submit (W6).
#
# One A2MCP service on POST /mcp. Listing copy is value-first (trust/install-risk);
# fee is only the structured fee field (FEE=0.1 USDT0 / tools/call, OKX Broker).
# Canonical text must stay in sync with docs/listings/directory-forms.md §OKX.
#
# Prerequisites:
#   1. onchainos CLI: curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
#   2. Agentic Wallet login:
#        onchainos wallet login <email>
#        onchainos wallet verify <OTP>
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

# Full-bleed 1:1 cover from official logo mark (no white canvas / rounded frame).
# Prefer cover for OKX.AI listing review; override with OKX_ASP_AVATAR if needed.
AVATAR="${OKX_ASP_AVATAR:-${ROOT}/public/brand/okx-ai-agent-cover.png}"
if [[ ! -f "${AVATAR}" ]]; then
  # Fallback: frontend tree (same asset, monorepo layout).
  AVATAR="${ROOT}/frontend/public/brand/okx-ai-agent-cover.png"
fi
if [[ ! -f "${AVATAR}" ]]; then
  echo "Avatar not found: ${AVATAR} (expected public/brand/okx-ai-agent-cover.png)" >&2
  exit 1
fi
echo "Using avatar: ${AVATAR}"

NAME="OnchainAI"
# Value-first — fee lives in structured fee field ($0.1), not the headline.
DESCRIPTION="Find, compare, and vet crypto MCP/CLI/SDK/API tools with trust scores and install-risk before your agent installs anything."
# Hybrid billing: free discovery lives at /mcp; OKX paid package is /mcp/okx only.
ENDPOINT="https://www.onchain-ai.xyz/mcp/okx"
FEE="0.1"
LANG="${OKX_ASP_LANG:-en-US}"
AGENT_ID="${OKX_ASP_AGENT_ID:-4609}"

SERVICE_NAME="OnchainAI MCP"
# ≤500 chars. No URLs (OKX D6). Two lines (OKX D1): capability summary + what caller provides.
# Value-first; fee is only in structured fee field.
SERVICE_DESCRIPTION=$'Crypto tool intelligence for AI agents: ranked search, trust and install-risk signals, side-by-side compare, install guides, x402 metadata, live endpoint probes, verified picks, and gap audits — so agents vet tools before they install or pay third parties. Maintained catalog, not a raw link dump.
Provide a JSON-RPC tools/call body (tool name plus arguments). If payment is required, settle the challenge and retry with a payment-signature header.'

# Single bundled A2MCP SKU for validate-listing / create.
SERVICE_JSON="$(python3 - <<PY
import json
print(json.dumps([{
    "serviceName": "${SERVICE_NAME}",
    "serviceDescription": """${SERVICE_DESCRIPTION}""",
    "serviceType": "A2MCP",
    "fee": "${FEE}",
    "endpoint": "${ENDPOINT}",
}]))
PY
)"

echo "== pre-check (asp) =="
precheck="$(onchainos agent pre-check --role asp)"
echo "${precheck}"

echo "== x402 endpoint check =="
# Hybrid package: GET /mcp/okx returns discovery JSON 200; unpaid tools/call is 402.
# Prefer live POST smoke (matches product) before optional onchainos probe.
smoke_code="$(curl -sS -o /tmp/okx_mcp_smoke.json -w '%{http_code}' -X POST "${ENDPOINT}" \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"bridge","limit":1}}}' \
  || true)"
echo "POST ${ENDPOINT} tools/call search_tools → HTTP ${smoke_code} (expect 402 when OKX gate active)"
if [[ "${smoke_code}" != "402" ]]; then
  echo "WARN: expected HTTP 402 on unpaid package tools/call; got ${smoke_code}" >&2
  head -c 400 /tmp/okx_mcp_smoke.json 2>/dev/null || true
  echo
fi
# onchainos x402-check often GETs the URL and flags MCP discovery 200 as invalid — advisory only.
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

echo "== validate-listing (1 bundled SKU @ ${FEE} USDT0) =="
validate_out="$(onchainos agent validate-listing --role asp --name "${NAME}" --description "${DESCRIPTION}" --service "${SERVICE_JSON}")"
echo "${validate_out}"
pass="$(echo "${validate_out}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass', False))" 2>/dev/null || echo false)"
if [[ "${pass}" != "True" && "${pass}" != "true" ]]; then
  echo "validate-listing failed — fix findings before submit." >&2
  exit 1
fi

if [[ -n "${AGENT_ID}" ]]; then
  echo "== fetch existing services for ASP #${AGENT_ID} =="
  service_list="$(onchainos agent service-list --agent-id "${AGENT_ID}" 2>/dev/null)" || true
  if [[ -z "${service_list}" ]] || ! echo "${service_list}" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
    echo "service-list failed for ASP #${AGENT_ID}" >&2
    exit 1
  fi
  echo "${service_list}" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('data', [{}])[0].get('list', [])
print(f'existing services: {len(items)}')
for s in items:
    print(f\"  delete id={s['id']} ({s['serviceName']})\")
"

  UPDATE_SERVICE_JSON="$(printf '%s' "${service_list}" | python3 -c "
import json, sys
name, desc, fee, endpoint = sys.argv[1:5]
items = json.load(sys.stdin).get('data', [{}])[0].get('list', [])
ops = []
for s in items:
    ops.append({
        'operation': 'delete',
        'id': str(s['id']),
        'serviceName': s['serviceName'],
        'serviceDescription': s['serviceDescription'],
        'serviceType': s['serviceType'],
        'fee': s['fee'],
        'endpoint': s['endpoint'],
    })
ops.append({
    'operation': 'create',
    'serviceName': name,
    'serviceDescription': desc,
    'serviceType': 'A2MCP',
    'fee': fee,
    'endpoint': endpoint,
})
print(json.dumps(ops))
" "${SERVICE_NAME}" "${SERVICE_DESCRIPTION}" "${FEE}" "${ENDPOINT}")"

  echo "== update ASP #${AGENT_ID} (replace with 1 bundled SKU) =="
  if ! update_out="$(onchainos agent update \
    --agent-id "${AGENT_ID}" \
    --name "${NAME}" \
    --description "${DESCRIPTION}" \
    --picture "${picture_url}" \
    --service "${UPDATE_SERVICE_JSON}" 2>&1)"; then
    echo "${update_out}" >&2
    exit 1
  fi
  echo "${update_out}"
else
  echo "== create ASP =="
  create_out="$(onchainos agent create \
    --role asp \
    --name "${NAME}" \
    --description "${DESCRIPTION}" \
    --picture "${picture_url}" \
    --service "${SERVICE_JSON}")"
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
echo "Done. ASP #${AGENT_ID} — 1 bundled A2MCP SKU @ ${FEE} USDT0/call on ${ENDPOINT}"
echo "Check okx.ai/agents and Agentic Wallet email for approval status."