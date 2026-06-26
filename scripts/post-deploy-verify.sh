#!/usr/bin/env bash
# Verify production after Railway deploy.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PROD_URL="${1:-https://www.onchain-ai.xyz}"
PROD_URL="${PROD_URL%/}"

echo "=== Production smoke (curl) ==="
./scripts/smoke-test.sh "${PROD_URL}"

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