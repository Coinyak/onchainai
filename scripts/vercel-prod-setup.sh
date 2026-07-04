#!/usr/bin/env bash
# One-shot Vercel production setup: disable SSO on production, set env, add domain.
#
# Prerequisites:
#   export VERCEL_TOKEN=...   # https://vercel.com/account/tokens
#   npx vercel login          # or token above
#
# Usage:
#   ./scripts/vercel-prod-setup.sh
#   ./scripts/vercel-prod-setup.sh --domain-only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/frontend"

TEAM_SLUG="${VERCEL_TEAM:-onchain-ai}"
PROJECT_NAME="${VERCEL_PROJECT:-onchainai}"
API_URL="${NEXT_PUBLIC_API_URL:-https://www.onchain-ai.xyz}"
API_PROXY="${API_PROXY_TARGET:-https://onchainai-production.up.railway.app}"
DOMAIN="${VERCEL_DOMAIN:-www.onchain-ai.xyz}"

if ! command -v npx >/dev/null 2>&1; then
  echo "npx required" >&2
  exit 1
fi

if ! npx vercel whoami >/dev/null 2>&1; then
  echo "Not logged in. Run: npx vercel login" >&2
  echo "Or export VERCEL_TOKEN from https://vercel.com/account/tokens" >&2
  exit 1
fi

echo "Linking frontend → ${TEAM_SLUG}/${PROJECT_NAME}..."
npx vercel link --yes --scope "${TEAM_SLUG}" --project "${PROJECT_NAME}" 2>/dev/null || \
  npx vercel link --yes --project "${PROJECT_NAME}"

if [[ "${1:-}" != "--domain-only" ]]; then
  echo "Disabling Vercel Authentication (SSO) on production deployments..."
  npx vercel project protection disable "${PROJECT_NAME}" --sso --scope "${TEAM_SLUG}" 2>/dev/null || \
    npx vercel project protection disable "${PROJECT_NAME}" --sso || true

  echo "Setting NEXT_PUBLIC_API_URL=${API_URL}..."
  printf '%s' "${API_URL}" | npx vercel env add NEXT_PUBLIC_API_URL production --scope "${TEAM_SLUG}" --force 2>/dev/null || \
    printf '%s' "${API_URL}" | npx vercel env add NEXT_PUBLIC_API_URL production --force || true
  echo "Removing NEXT_PUBLIC_API_URL from preview (same-origin rewrites; avoids CORS/session breaks)..."
  npx vercel env rm NEXT_PUBLIC_API_URL preview --scope "${TEAM_SLUG}" --yes 2>/dev/null || \
    npx vercel env rm NEXT_PUBLIC_API_URL preview --yes 2>/dev/null || true

  echo "Setting API_PROXY_TARGET=${API_PROXY} (auth/onboarding/mcp rewrites)..."
  printf '%s' "${API_PROXY}" | npx vercel env add API_PROXY_TARGET production --scope "${TEAM_SLUG}" --force 2>/dev/null || \
    printf '%s' "${API_PROXY}" | npx vercel env add API_PROXY_TARGET production --force || true
  printf '%s' "${API_PROXY}" | npx vercel env add API_PROXY_TARGET preview --scope "${TEAM_SLUG}" --force 2>/dev/null || \
    printf '%s' "${API_PROXY}" | npx vercel env add API_PROXY_TARGET preview --force || true
fi

echo "Adding domain ${DOMAIN} (DNS must point to Vercel before it goes live)..."
npx vercel domains add "${DOMAIN}" --scope "${TEAM_SLUG}" 2>/dev/null || \
  npx vercel domains add "${DOMAIN}" || true

echo "Production deploy..."
npx vercel deploy --prod --yes --scope "${TEAM_SLUG}"

echo ""
echo "Done. Your deployment URL (team-scoped):"
npx vercel inspect --scope "${TEAM_SLUG}" 2>/dev/null | head -5 || true
echo ""
echo "NOT your site: https://onchainai.vercel.app (global name — another project)"
echo "Your team URLs: https://${PROJECT_NAME}-${TEAM_SLUG}.vercel.app or deployment hash URL"
echo "Current Railway site: ${API_URL}"