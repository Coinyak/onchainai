#!/usr/bin/env bash
# Ensure committed MCP configs agree on server keys and Vercel URL.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CANONICAL_SERVERS="onchainai railway vercel"
CANONICAL_VERCEL_URL="https://mcp.vercel.com/onchain-ai/onchainai"

MCP_JSON="${ROOT}/.mcp.json"
CURSOR_MCP_JSON="${ROOT}/.cursor/mcp.json"
GROK_CONFIG="${ROOT}/.grok/config.toml"

fail() {
  echo "MCP CONFIG PARITY FAIL: $*" >&2
  exit 1
}

json_server_keys() {
  node -e '
    const fs = require("fs");
    const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const keys = Object.keys(data.mcpServers || {}).sort();
    process.stdout.write(keys.join(" "));
  ' "$1"
}

json_vercel_url() {
  node -e '
    const fs = require("fs");
    const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const vercel = (data.mcpServers || {}).vercel || {};
    process.stdout.write(vercel.url || vercel.serverUrl || "");
  ' "$1"
}

grok_server_keys() {
  grep -E '^\[mcp_servers\.' "$1" \
    | sed -E 's/^\[mcp_servers\.([^]]+)\].*/\1/' \
    | sort \
    | tr '\n' ' ' \
    | sed 's/ $//'
}

grok_vercel_url() {
  awk '
    /^\[mcp_servers\.vercel\]/ { in_vercel = 1; next }
    /^\[/ { in_vercel = 0 }
    in_vercel && $1 == "url" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$1"
}

for path in "$MCP_JSON" "$CURSOR_MCP_JSON" "$GROK_CONFIG"; do
  [[ -f "$path" ]] || fail "missing config file: ${path}"
done

mcp_keys="$(json_server_keys "$MCP_JSON")"
cursor_keys="$(json_server_keys "$CURSOR_MCP_JSON")"
grok_keys="$(grok_server_keys "$GROK_CONFIG")"

if [[ "$mcp_keys" != "$CANONICAL_SERVERS" ]]; then
  fail ".mcp.json servers mismatch (expected: ${CANONICAL_SERVERS}, got: ${mcp_keys})"
fi
if [[ "$cursor_keys" != "$CANONICAL_SERVERS" ]]; then
  fail ".cursor/mcp.json servers mismatch (expected: ${CANONICAL_SERVERS}, got: ${cursor_keys})"
fi
if [[ "$grok_keys" != "$CANONICAL_SERVERS" ]]; then
  fail ".grok/config.toml servers mismatch (expected: ${CANONICAL_SERVERS}, got: ${grok_keys})"
fi

check_vercel_url() {
  local file="$1"
  local url="$2"
  if [[ "$url" != "$CANONICAL_VERCEL_URL" ]]; then
    fail "${file} vercel URL mismatch (expected: ${CANONICAL_VERCEL_URL}, got: ${url:-<empty>})"
  fi
}

check_vercel_url ".mcp.json" "$(json_vercel_url "$MCP_JSON")"
check_vercel_url ".cursor/mcp.json" "$(json_vercel_url "$CURSOR_MCP_JSON")"
check_vercel_url ".grok/config.toml" "$(grok_vercel_url "$GROK_CONFIG")"

echo "MCP CONFIG PARITY PASS (servers: ${CANONICAL_SERVERS}; vercel: ${CANONICAL_VERCEL_URL})"