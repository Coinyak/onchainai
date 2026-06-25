#!/usr/bin/env bash
# Apply GitHub OAuth + redirect URLs to Supabase Auth via Management API.
# Requires: SUPABASE_ACCESS_TOKEN (sbp_...) from https://supabase.com/dashboard/account/tokens
#
# Usage:
#   ./scripts/configure-supabase-auth.sh          # localhost (default)
#   ONCHAINAI_ENV=prod ./scripts/configure-supabase-auth.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

: "${SUPABASE_ACCESS_TOKEN:?Set SUPABASE_ACCESS_TOKEN (sbp_... from Supabase Account > Access Tokens)}"
: "${GITHUB_CLIENT_ID:?Set GITHUB_CLIENT_ID in .env}"
: "${GITHUB_CLIENT_SECRET:?Set GITHUB_CLIENT_SECRET in .env}"

PROJECT_REF="puvxrdsgexjxvgfiepua"

if [[ "${ONCHAINAI_ENV:-dev}" == "prod" ]]; then
  export SITE_URL="${SITE_URL:-https://onchainai.xyz}"
  export REDIRECT_URLS="${REDIRECT_URLS:-https://onchainai.xyz/auth/callback,http://localhost:3000/auth/callback}"
else
  export SITE_URL="${SITE_URL:-http://localhost:3000}"
  export REDIRECT_URLS="${REDIRECT_URLS:-http://localhost:3000/auth/callback}"
fi

BODY=$(/usr/bin/python3 -c "
import json, os
print(json.dumps({
    'external_github_enabled': True,
    'external_github_client_id': os.environ['GITHUB_CLIENT_ID'],
    'external_github_secret': os.environ['GITHUB_CLIENT_SECRET'],
    'site_url': os.environ['SITE_URL'],
    'uri_allow_list': os.environ['REDIRECT_URLS'],
}))
")

echo "Updating Supabase Auth config for project ${PROJECT_REF} (${ONCHAINAI_ENV:-dev})..."
echo "  site_url=${SITE_URL}"
echo "  uri_allow_list=${REDIRECT_URLS}"

HTTP_CODE=$(/usr/bin/curl -s -o /tmp/supabase-auth-response.json -w "%{http_code}" -X PATCH \
  -H "Authorization: Bearer ${SUPABASE_ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "${BODY}" \
  "https://api.supabase.com/v1/projects/${PROJECT_REF}/config/auth")

if [[ "${HTTP_CODE}" != "200" ]]; then
  echo "Failed (HTTP ${HTTP_CODE}):" >&2
  cat /tmp/supabase-auth-response.json >&2
  exit 1
fi

echo "OK — GitHub provider + redirect URLs applied."
/usr/bin/curl -s \
  -H "apikey: ${SUPABASE_SERVICE_KEY}" \
  -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
  "${SUPABASE_URL}/auth/v1/settings" | /usr/bin/python3 -c "import sys,json; d=json.load(sys.stdin); print('github enabled:', d.get('external',{}).get('github'))"