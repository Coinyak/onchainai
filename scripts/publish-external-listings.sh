#!/usr/bin/env bash
# Operator runbook: repo-side listing prep + optional prod catalog seed.
# Does NOT submit web forms (Smithery/mcp.so) — opens copy in docs/listings/directory-forms.md
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== OnchainAI external listings runbook =="

echo "[1/5] Validate server.json"
python3 -m json.tool server.json >/dev/null
echo "  OK: server.json"

echo "[2/5] Claude plugin validate"
if command -v claude >/dev/null 2>&1; then
  claude plugin validate plugin/onchainai
else
  echo "  SKIP: claude CLI not installed"
fi

echo "[3/5] Dry-run catalog self-list seed"
node scripts/seed-onchainai-listing.mjs

if [[ "${SEED_APPLY:-}" == "1" ]]; then
  echo "[4/5] APPLY prod catalog seed (SEED_APPLY=1)"
  ENV_FILE="${ENV_FILE:-.env}" SEED_ENV=prod-curate PG_INSECURE_SSL="${PG_INSECURE_SSL:-1}" \
    node scripts/seed-onchainai-listing.mjs
else
  echo "[4/5] Skip prod seed (set SEED_APPLY=1 to apply)"
fi

echo "[5/5] MCP Registry publish (manual — needs DNS TXT + mcp-publisher)"
echo "  See docs/listings/directory-forms.md"
echo "  server.json name: xyz.onchain-ai/onchainai"
echo ""
echo "Directory form copy: docs/listings/directory-forms.md"
echo "PR payloads: docs/listings/web3-mcp-hub-onchainai.json"
echo "             docs/listings/awesome-crypto-mcp-servers.md"
echo ""
echo "Done."