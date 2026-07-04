#!/usr/bin/env bash
# Publish OnchainAI to the official MCP Registry.
# Auth: GitHub device flow (name must be io.github.Coinyak/onchainai) OR HTTP /.well-known (see setup-mcp-registry-http.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUBLISHER="${ROOT}/bin/mcp-publisher"
SERVER_JSON="${ROOT}/server.json"

if [[ ! -x "${PUBLISHER}" ]]; then
  echo "Installing mcp-publisher..."
  ARCH="$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')"
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  mkdir -p "${ROOT}/bin"
  curl -sL "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_${OS}_${ARCH}.tar.gz" \
    | tar xzf - -C "${ROOT}/bin" mcp-publisher
  chmod +x "${PUBLISHER}"
fi

python3 -m json.tool "${SERVER_JSON}" >/dev/null
"${PUBLISHER}" validate "${SERVER_JSON}"

NAME="$(python3 -c 'import json;print(json.load(open("'"${SERVER_JSON}"'"))["name"])')"

PUBLISH_EC=0
PUBLISH_OUT="$("${PUBLISHER}" publish "${SERVER_JSON}" 2>&1)" || PUBLISH_EC=$?

if [[ "${PUBLISH_EC}" -eq 0 ]]; then
  echo "${PUBLISH_OUT}"
elif echo "${PUBLISH_OUT}" | grep -qiE 'duplicate version|already published'; then
  echo "${PUBLISH_OUT}"
  echo ""
  echo "Already published at this version — bump version in server.json to publish metadata changes."
  exit 0
elif [[ "${NAME}" == io.github.* ]] \
  && echo "${PUBLISH_OUT}" | grep -qiE '401|unauthorized|not logged in|invalid or expired|authentication'; then
  echo "${PUBLISH_OUT}"
  echo ""
  echo "GitHub auth required for ${NAME}"
  echo "Visit https://github.com/login/device when prompted and authorize."
  "${PUBLISHER}" login github
  "${PUBLISHER}" publish "${SERVER_JSON}"
elif [[ "${NAME}" == io.github.* ]]; then
  echo "${PUBLISH_OUT}"
  exit "${PUBLISH_EC}"
else
  echo "${PUBLISH_OUT}"
  echo ""
  echo "Domain auth required for ${NAME}"
  echo "  HTTP: apex must serve /.well-known/mcp-registry-auth without 307 (or use DNS TXT)"
  echo "  Run: ./scripts/godaddy-mcp-registry-txt.sh  OR  ./scripts/publish-mcp-registry-dns.sh"
  exit 1
fi

echo ""
echo "Verify:"
echo "  curl -s 'https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.Coinyak/onchainai' | python3 -m json.tool | head -40"