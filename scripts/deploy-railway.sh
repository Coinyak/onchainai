#!/usr/bin/env bash
# Deploy OnchainAI to Railway (Dockerfile) and sync production env vars.
#
# Prerequisites:
#   railway login          # or RAILWAY_TOKEN from https://railway.com/account/tokens
#   .env with DATABASE_URL, Supabase keys, GitHub OAuth, JWT_SECRET
#
# Usage:
#   ./scripts/deploy-railway.sh                    # sync vars + deploy (main only)
#   ./scripts/deploy-railway.sh --vars-only          # sync env vars only (no deploy)
#   ./scripts/deploy-railway.sh --force-non-main     # emergency: deploy from current branch
#
# Normal prod path: merge to main → Railway GitHub deploy (watchPatterns in railway.json).
# See docs/superpowers/specs/2026-07-05-split-deploy-automation-spec.md
#
# `railway up` is called with an explicit `"${ROOT}" --path-as-root` — do not
# drop this. `~/.railway/config.json` links this project to a fixed directory
# path (whichever checkout first ran `railway link`); if that path differs
# from where this script lives (e.g. running from a `git worktree` checkout),
# a bare `railway up` silently uploads/builds from the *linked* path instead
# of the caller's cwd, producing a stale image with no error.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DEPLOY_BRANCH=""
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  DEPLOY_BRANCH="$(git branch --show-current 2>/dev/null || true)"
fi

VARS_ONLY=false
FORCE_NON_MAIN=false
for arg in "$@"; do
  case "${arg}" in
    --vars-only) VARS_ONLY=true ;;
    --force-non-main) FORCE_NON_MAIN=true ;;
  esac
done

echo "Git branch: ${DEPLOY_BRANCH:-unknown}"

if [[ "${VARS_ONLY}" != true && "${FORCE_NON_MAIN}" != true && -n "${DEPLOY_BRANCH}" && "${DEPLOY_BRANCH}" != "main" ]]; then
  echo "Refusing production deploy from branch '${DEPLOY_BRANCH}'." >&2
  echo "Merge to main and push — Railway auto-deploys API when watchPatterns match." >&2
  echo "Vercel already previews frontend on every push." >&2
  echo "Env sync only: ./scripts/deploy-railway.sh --vars-only" >&2
  echo "Emergency override: ./scripts/deploy-railway.sh --force-non-main" >&2
  exit 1
fi

if [[ "${FORCE_NON_MAIN}" == true && -n "${DEPLOY_BRANCH}" && "${DEPLOY_BRANCH}" != "main" ]]; then
  echo "WARNING: deploying non-main branch '${DEPLOY_BRANCH}' to production. Merge to main ASAP." >&2
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
export GITHUB_CLIENT_ID="${GITHUB_CLIENT_ID_PROD:-${GITHUB_CLIENT_ID}}"
export GITHUB_CLIENT_SECRET="${GITHUB_CLIENT_SECRET_PROD:-${GITHUB_CLIENT_SECRET}}"
export GITHUB_REDIRECT_URI="${GITHUB_REDIRECT_URI_PROD:-https://${SIWX_DOMAIN}/auth/callback}"
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

if ! RAILWAY_USER="$(railway whoami 2>/dev/null)"; then
  echo "Not logged in. Run: railway login" >&2
  echo "Or set RAILWAY_TOKEN from https://railway.com/account/tokens" >&2
  exit 1
fi
echo "Railway user: ${RAILWAY_USER}"

SERVICE_NAME="${RAILWAY_SERVICE:-onchainai}"

if ! railway status >/dev/null 2>&1; then
  echo "No linked Railway project. Run: railway link -p <project-id> -s ${SERVICE_NAME} -e production" >&2
  echo "Or create a new project: railway init --name ${SERVICE_NAME}" >&2
  exit 1
fi

sync_vars() {
  echo "Syncing environment variables (SIWX_DOMAIN=${SIWX_DOMAIN})..."
  vars=(
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
    "RUST_LOG=${RUST_LOG}" \
    "DATABASE_MAX_CONNECTIONS=${DATABASE_MAX_CONNECTIONS:-10}" \
    "SKIP_CRAWLER=${SKIP_CRAWLER:-1}" \
    "RUST_MIN_STACK=${RUST_MIN_STACK:-8388608}"
  )
  if [[ -n "${ADMIN_GITHUB_LOGINS:-}" ]]; then
    vars+=("ADMIN_GITHUB_LOGINS=${ADMIN_GITHUB_LOGINS}")
  fi
  if [[ -n "${GITHUB_REDIRECT_URI:-}" ]]; then
    vars+=("GITHUB_REDIRECT_URI=${GITHUB_REDIRECT_URI}")
  fi
  if [[ -n "${GOOGLE_CLIENT_ID:-}" ]]; then
    vars+=("GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}")
  fi
  if [[ -n "${GOOGLE_CLIENT_SECRET:-}" ]]; then
    vars+=("GOOGLE_CLIENT_SECRET=${GOOGLE_CLIENT_SECRET}")
  fi
  if [[ -n "${GOOGLE_REDIRECT_URI:-}" ]]; then
    vars+=("GOOGLE_REDIRECT_URI=${GOOGLE_REDIRECT_URI}")
  fi
  if [[ -n "${X402_FACILITATOR_URL:-}" ]]; then
    vars+=("X402_FACILITATOR_URL=${X402_FACILITATOR_URL}")
  fi
  if [[ -n "${X402_PAY_TO_ADDRESS:-}" ]]; then
    vars+=("X402_PAY_TO_ADDRESS=${X402_PAY_TO_ADDRESS}")
  fi
  if [[ -n "${X402_NETWORK:-}" ]]; then
    vars+=("X402_NETWORK=${X402_NETWORK}")
  fi
  if [[ -n "${X402_PREMIUM_PRICE_USD:-}" ]]; then
    # Quote so literal "$0.001" is not expanded by bash or the Railway CLI.
    vars+=('X402_PREMIUM_PRICE_USD='"${X402_PREMIUM_PRICE_USD}")
  fi
  if [[ -n "${CDP_API_KEY_NAME:-}" ]]; then
    vars+=("CDP_API_KEY_NAME=${CDP_API_KEY_NAME}")
  fi
  if [[ -n "${CDP_API_KEY_PRIVATE_KEY:-}" ]]; then
    vars+=("CDP_API_KEY_PRIVATE_KEY=${CDP_API_KEY_PRIVATE_KEY}")
  fi
  railway variable set -s "${SERVICE_NAME}" --skip-deploys "${vars[@]}"
}

if [[ "${VARS_ONLY}" == true ]]; then
  sync_vars
  echo "Variables synced (--vars-only)."
  exit 0
fi

# First deploy: Railway needs a service before `variable set` works. We create it with
# `railway up`, sync vars with --skip-deploys, then deploy again so the image boots
# with production env (not the empty/default first boot).
if ! railway status --json 2>/dev/null | /usr/bin/grep -q '"services"'; then
  echo "Initial deploy (creates service; production env applied on the next up)..."
  railway up "${ROOT}" --path-as-root -y --detach -s "${SERVICE_NAME}"
fi

sync_vars

echo "Deploying from Dockerfile with production env..."
railway up "${ROOT}" --path-as-root -y --detach -s "${SERVICE_NAME}"

DEPLOY_WAIT_ATTEMPTS="${DEPLOY_WAIT_ATTEMPTS:-120}"
DEPLOY_WAIT_INTERVAL="${DEPLOY_WAIT_INTERVAL:-5}"
DEPLOY_SMOKE_ATTEMPTS="${DEPLOY_SMOKE_ATTEMPTS:-30}"
DEPLOY_SMOKE_INTERVAL="${DEPLOY_SMOKE_INTERVAL:-5}"

latest_deployment_status() {
  railway deployment list --json -s "${SERVICE_NAME}" --limit 1 2>/dev/null \
    | /usr/bin/python3 -c "import json,sys; d=json.load(sys.stdin); print(d[0]['status'] if d else '')" 2>/dev/null \
    || true
}

railway_api_url() {
  if [[ -n "${RAILWAY_API_URL:-}" ]]; then
    printf '%s' "${RAILWAY_API_URL%/}"
    return 0
  fi
  local resolved
  resolved="$(railway status --json 2>/dev/null \
    | /usr/bin/python3 -c "import json,sys
d=json.load(sys.stdin)
for env in d.get('environments',{}).get('edges',[]):
  for edge in env['node'].get('serviceInstances',{}).get('edges',[]):
    node=edge['node']
    for dom in (node.get('domains',{}) or {}).get('serviceDomains',[]) or []:
      if dom.get('domain'):
        print(dom['domain']); raise SystemExit
" 2>/dev/null || true)"
  if [[ -n "$resolved" ]]; then
    printf 'https://%s' "${resolved%/}"
    return 0
  fi
  printf '%s' "https://onchainai-production.up.railway.app"
}

echo "Waiting for Railway deployment to succeed (up to ${DEPLOY_WAIT_ATTEMPTS} checks, ${DEPLOY_WAIT_INTERVAL}s interval)..."
for attempt in $(seq 1 "${DEPLOY_WAIT_ATTEMPTS}"); do
  deploy_status="$(latest_deployment_status)"
  case "${deploy_status}" in
    SUCCESS)
      echo "Railway deployment succeeded."
      break
      ;;
    FAILED|CRASHED|REMOVED)
      echo "Railway deployment failed (status=${deploy_status})." >&2
      echo "Inspect: railway deployment list && railway logs" >&2
      exit 1
      ;;
    *)
      if [[ "${attempt}" -eq "${DEPLOY_WAIT_ATTEMPTS}" ]]; then
        echo "Timed out waiting for Railway deployment (last status=${deploy_status:-unknown})." >&2
        exit 1
      fi
      # Only log every 4th check (~20s) to reduce noise.
      if [[ $((attempt % 4)) -eq 0 ]]; then
        echo "  ... still ${deploy_status:-building} (${attempt}/${DEPLOY_WAIT_ATTEMPTS})"
      fi
      sleep "${DEPLOY_WAIT_INTERVAL}"
      ;;
  esac
done

API_URL="$(railway_api_url)"
echo "Waiting for Railway API smoke (${API_URL})..."
for attempt in $(seq 1 "${DEPLOY_SMOKE_ATTEMPTS}"); do
  if ./scripts/smoke-test-api.sh "${API_URL}" 2>/dev/null; then
    echo "Railway API smoke passed."
    break
  fi
  if [[ "${attempt}" -eq "${DEPLOY_SMOKE_ATTEMPTS}" ]]; then
    echo "Railway API smoke failed after ${DEPLOY_SMOKE_ATTEMPTS} attempts." >&2
    echo "The API may still be starting. Check: curl -sS -o /dev/null -w '%{http_code}' ${API_URL}/favicon.ico" >&2
    exit 1
  fi
  sleep "${DEPLOY_SMOKE_INTERVAL}"
done

echo ""
echo "Next steps:"
echo "  1. Add custom domain: railway domain add www.onchain-ai.xyz"
echo "  2. GitHub OAuth App callback: https://www.onchain-ai.xyz/auth/callback"
echo "  3. Supabase prod URLs: ONCHAINAI_ENV=prod SUPABASE_ACCESS_TOKEN=sbp_... ./scripts/configure-supabase-auth.sh"
echo "  4. Full verify (curl + browser): ./scripts/post-deploy-verify.sh https://${SIWX_DOMAIN}"
echo "  5. Admin crawl: https://www.onchain-ai.xyz/admin/crawler"
