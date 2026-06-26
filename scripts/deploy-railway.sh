#!/usr/bin/env bash
# Deploy OnchainAI to Railway (Dockerfile) and sync production env vars.
#
# Prerequisites:
#   railway login          # or RAILWAY_TOKEN from https://railway.com/account/tokens
#   .env with DATABASE_URL, Supabase keys, GitHub OAuth, JWT_SECRET
#
# Usage:
#   ./scripts/deploy-railway.sh              # link/create project + push vars + deploy
#   ./scripts/deploy-railway.sh --vars-only  # sync env vars only (no deploy)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VARS_ONLY=false
if [[ "${1:-}" == "--vars-only" ]]; then
  VARS_ONLY=true
fi

if ! command -v railway >/dev/null 2>&1; then
  echo "Install Railway CLI: npm i -g @railway/cli" >&2
  exit 1
fi

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

# Production overrides (never commit these values).
export SIWX_DOMAIN="${SIWX_DOMAIN_PROD:-www.onchain-ai.xyz}"
export PORT="${PORT:-3000}"
export RUST_LOG="${RUST_LOG:-info}"

if [[ -z "${GITHUB_API_TOKEN:-}" || "${GITHUB_API_TOKEN}" == *placeholder* ]]; then
  if command -v gh >/dev/null 2>&1; then
    GITHUB_API_TOKEN="$(gh auth token)"
    export GITHUB_API_TOKEN
    echo "Using gh auth token for GITHUB_API_TOKEN"
  else
    echo "Set GITHUB_API_TOKEN in .env or install gh CLI" >&2
    exit 1
  fi
fi

: "${DATABASE_URL:?Set DATABASE_URL in .env}"
: "${SUPABASE_URL:?Set SUPABASE_URL in .env}"
: "${SUPABASE_ANON_KEY:?Set SUPABASE_ANON_KEY in .env}"
: "${SUPABASE_SERVICE_KEY:?Set SUPABASE_SERVICE_KEY in .env}"
: "${GITHUB_CLIENT_ID:?Set GITHUB_CLIENT_ID in .env}"
: "${GITHUB_CLIENT_SECRET:?Set GITHUB_CLIENT_SECRET in .env}"
: "${JWT_SECRET:?Set JWT_SECRET in .env}"

if ! railway whoami >/dev/null 2>&1; then
  echo "Not logged in. Run: railway login" >&2
  echo "Or set RAILWAY_TOKEN from https://railway.com/account/tokens" >&2
  exit 1
fi

echo "Railway user: $(railway whoami)"

SERVICE_NAME="${RAILWAY_SERVICE:-onchainai}"

if [[ ! -f .railway/project.json ]]; then
  echo "Creating Railway project ${SERVICE_NAME}..."
  railway init --name "${SERVICE_NAME}"
fi

sync_vars() {
  echo "Syncing environment variables (SIWX_DOMAIN=${SIWX_DOMAIN})..."
  railway variable set -s "${SERVICE_NAME}" --skip-deploys \
    "DATABASE_URL=${DATABASE_URL}" \
    "SUPABASE_URL=${SUPABASE_URL}" \
    "SUPABASE_ANON_KEY=${SUPABASE_ANON_KEY}" \
    "SUPABASE_SERVICE_KEY=${SUPABASE_SERVICE_KEY}" \
    "GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID}" \
    "GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET}" \
    "GITHUB_API_TOKEN=${GITHUB_API_TOKEN}" \
    "SIWX_DOMAIN=${SIWX_DOMAIN}" \
    "SIWX_SESSION_TTL=${SIWX_SESSION_TTL:-86400}" \
    "JWT_SECRET=${JWT_SECRET}" \
    "PORT=${PORT}" \
    "RUST_LOG=${RUST_LOG}"
}

if [[ "${VARS_ONLY}" == true ]]; then
  sync_vars
  echo "Variables synced (--vars-only)."
  exit 0
fi

# First deploy creates the service; env vars are applied before the production deploy.
if ! railway status --json 2>/dev/null | /usr/bin/grep -q '"services"'; then
  echo "Initial deploy (creates service)..."
  railway up -y --detach -s "${SERVICE_NAME}"
fi

sync_vars

echo "Deploying from Dockerfile with production env..."
railway up -y --detach -s "${SERVICE_NAME}"

echo "Waiting for production deployment to become reachable..."
PROD_URL="https://www.onchain-ai.xyz"
for attempt in $(seq 1 30); do
  if ./scripts/smoke-test.sh "${PROD_URL}"; then
    echo "Production smoke passed."
    break
  fi
  if [[ "${attempt}" -eq 30 ]]; then
    echo "Production smoke failed after 30 attempts." >&2
    exit 1
  fi
  echo "Smoke attempt ${attempt}/30 failed; retrying in 10s..."
  sleep 10
done

echo ""
echo "Next steps:"
echo "  1. Add custom domain: railway domain add www.onchain-ai.xyz"
echo "  2. GitHub OAuth App callback: https://www.onchain-ai.xyz/auth/callback"
echo "  3. Supabase prod URLs: ONCHAINAI_ENV=prod SUPABASE_ACCESS_TOKEN=sbp_... ./scripts/configure-supabase-auth.sh"
echo "  4. Smoke test: ./scripts/smoke-test.sh https://www.onchain-ai.xyz"
echo "  5. Admin crawl: https://www.onchain-ai.xyz/admin/crawler"