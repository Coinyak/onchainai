#!/usr/bin/env bash
# Verify production after Railway deploy (curl smoke + optional Playwright).
#
# deploy-railway.sh already retries API curl smoke before exit; use this script for
# frontend + API checks and browser tests (browser-smoke.mjs, click-test.mjs).
#
# Usage:
#   ./scripts/post-deploy-verify.sh
#   ./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
#   ./scripts/post-deploy-verify.sh --k2
#   ./scripts/post-deploy-verify.sh --wave2
#   RAILWAY_API_URL=https://onchainai-production.up.railway.app ./scripts/post-deploy-verify.sh --k2
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PROD_URL="https://www.onchain-ai.xyz"
RUN_K2=false
RUN_WAVE2=false
for arg in "$@"; do
  case "$arg" in
    --k2) RUN_K2=true ;;
    --wave2) RUN_WAVE2=true ;;
    -*) echo "Unknown flag: $arg" >&2; exit 1 ;;
    *) PROD_URL="$arg" ;;
  esac
done
PROD_URL="${PROD_URL%/}"
API_URL="${RAILWAY_API_URL:-https://onchainai-production.up.railway.app}"
API_URL="${API_URL%/}"

echo "=== Production frontend smoke (curl) ==="
./scripts/smoke-test-frontend.sh "${PROD_URL}"

echo "=== Production API smoke (curl) ==="
./scripts/smoke-test-api.sh "${API_URL}"

if [[ "$RUN_K2" == "true" ]]; then
  echo "=== K2 prod smoke (discovery free + check_endpoint_health 402) ==="
  RAILWAY_API_URL="${API_URL}" ./scripts/k2-prod-smoke.sh
fi

if [[ "$RUN_WAVE2" == "true" ]]; then
  echo "=== Wave 2 prod verify (W3 + W8; L4 SQL when DATABASE_URL set) ==="
  RAILWAY_API_URL="${API_URL}" ONCHAINAI_FRONTEND_URL="${PROD_URL}" ./scripts/wave2-prod-verify.sh
fi

if command -v node >/dev/null 2>&1; then
  if node -e "require.resolve('playwright')" >/dev/null 2>&1; then
    if [[ -f scripts/browser-smoke.mjs ]]; then
      echo "=== Production browser smoke ==="
      node scripts/browser-smoke.mjs "${PROD_URL}"
    fi
    if [[ -f scripts/click-test.mjs ]]; then
      echo "=== Production click test ==="
      node scripts/click-test.mjs "${PROD_URL}"
    fi
  else
    echo "Skip browser tests: npm install --no-save playwright && npx playwright install chromium"
  fi
fi

echo "POST-DEPLOY VERIFY PASS ${PROD_URL}"