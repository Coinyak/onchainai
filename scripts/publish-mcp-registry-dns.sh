#!/usr/bin/env bash
# Publish after DNS TXT propagation (domain auth, name xyz.onchain-ai/onchainai).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUBLISHER="${ROOT}/bin/mcp-publisher"
SERVER_JSON="${ROOT}/server.json"
DOMAIN="${MCP_REGISTRY_DOMAIN:-onchain-ai.xyz}"
OPENSSL="${OPENSSL_BIN:-$HOME/.local/openssl/bin/openssl}"
KEY_PEM="${ROOT}/.mcp-registry/ed25519-key.pem"

python3 -m json.tool "${SERVER_JSON}" >/dev/null
grep -q '"xyz.onchain-ai/onchainai"' "${SERVER_JSON}" || {
  echo "server.json name must be xyz.onchain-ai/onchainai for DNS auth"
  exit 1
}

PRIVATE_KEY="$("${OPENSSL}" pkey -in "${KEY_PEM}" -noout -text | grep -A3 'priv:' | tail -n +2 | tr -d ' :\n')"
"${PUBLISHER}" login dns --domain "${DOMAIN}" --private-key "${PRIVATE_KEY}"
"${PUBLISHER}" publish "${SERVER_JSON}"
echo "curl -s 'https://registry.modelcontextprotocol.io/v0.1/servers?search=xyz.onchain-ai/onchainai' | python3 -m json.tool | head -30"