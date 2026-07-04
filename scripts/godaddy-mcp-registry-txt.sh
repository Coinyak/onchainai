#!/usr/bin/env bash
# Add MCP Registry DNS proof TXT on apex onchain-ai.xyz (GoDaddy API).
# Prereq: export GODADDY_API_KEY / GODADDY_API_SECRET
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOMAIN="${GODADDY_DOMAIN:-onchain-ai.xyz}"
HOST="${MCP_TXT_HOST:-@}"
OPENSSL="${OPENSSL_BIN:-$HOME/.local/openssl/bin/openssl}"
KEY_PEM="${ROOT}/.mcp-registry/ed25519-key.pem"

: "${GODADDY_API_KEY:?Set GODADDY_API_KEY from https://developer.godaddy.com/keys}"
: "${GODADDY_API_SECRET:?Set GODADDY_API_SECRET}"

if [[ ! -f "${KEY_PEM}" ]]; then
  "${ROOT}/scripts/setup-mcp-registry-http.sh"
fi

PUBLIC_KEY="$("${OPENSSL}" pkey -in "${KEY_PEM}" -pubout -outform DER | tail -c 32 | base64)"
TXT_VALUE="v=MCPv1; k=ed25519; p=${PUBLIC_KEY}"
AUTH="sso-key ${GODADDY_API_KEY}:${GODADDY_API_SECRET}"
API="https://api.godaddy.com/v1/domains/${DOMAIN}/records/TXT/${HOST}"

echo "Setting TXT ${HOST}.${DOMAIN} ..."
HTTP=$(curl -sS -o /tmp/gd-mcp-txt.json -w "%{http_code}" -X PUT \
  -H "Authorization: ${AUTH}" -H "Content-Type: application/json" \
  -d "[{\"data\":\"${TXT_VALUE}\",\"ttl\":600}]" \
  "${API}")
echo "HTTP ${HTTP}"
cat /tmp/gd-mcp-txt.json 2>/dev/null; echo
echo "Verify (wait 2-10 min): dig +short ${DOMAIN} TXT"
echo "Then: ${ROOT}/scripts/publish-mcp-registry-dns.sh"