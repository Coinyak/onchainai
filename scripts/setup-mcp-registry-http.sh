#!/usr/bin/env bash
# HTTP auth for MCP Registry (alternative to GitHub login).
# Hosts public proof at https://www.onchain-ai.xyz/.well-known/mcp-registry-auth
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOMAIN="${MCP_REGISTRY_DOMAIN:-onchain-ai.xyz}"
KEY_DIR="${ROOT}/.mcp-registry"
KEY_PEM="${KEY_DIR}/ed25519-key.pem"
WELL_KNOWN="${ROOT}/frontend/public/.well-known/mcp-registry-auth"
OPENSSL="${OPENSSL_BIN:-$HOME/.local/openssl/bin/openssl}"
if [[ ! -x "${OPENSSL}" ]]; then
  OPENSSL="openssl"
fi

mkdir -p "${KEY_DIR}" "${ROOT}/frontend/public/.well-known"
if [[ ! -f "${KEY_PEM}" ]]; then
  "${OPENSSL}" genpkey -algorithm Ed25519 -out "${KEY_PEM}"
  chmod 600 "${KEY_PEM}"
fi

PUBLIC_KEY="$("${OPENSSL}" pkey -in "${KEY_PEM}" -pubout -outform DER | tail -c 32 | base64)"
echo "v=MCPv1; k=ed25519; p=${PUBLIC_KEY}" > "${WELL_KNOWN}"
echo "Wrote ${WELL_KNOWN}"
echo ""
echo "DNS TXT (optional alternative to HTTP file) for ${DOMAIN}:"
echo "  ${DOMAIN}. IN TXT \"v=MCPv1; k=ed25519; p=${PUBLIC_KEY}\""
echo ""
echo "After Vercel deploy, login + publish:"
echo "  PRIVATE_KEY=\$(${OPENSSL} pkey -in ${KEY_PEM} -noout -text | grep -A3 'priv:' | tail -n +2 | tr -d ' :\\n')"
echo "  ${ROOT}/bin/mcp-publisher login http --domain ${DOMAIN} --private-key \"\${PRIVATE_KEY}\""
echo "  # Update server.json name to xyz.onchain-ai/onchainai then publish"