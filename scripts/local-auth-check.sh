#!/usr/bin/env bash
# Verify local GitHub OAuth callback URL: .env expectation vs running server.
#
# Usage:
#   ./scripts/local-auth-check.sh
#   ./scripts/local-auth-check.sh http://localhost:3000
#
# Does not print secrets from .env — only SIWX_DOMAIN, PORT, GITHUB_REDIRECT_URI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BASE="${1:-http://localhost:3000}"
BASE="${BASE%/}"

fail() {
  echo "AUTH CHECK FAIL: $*" >&2
  exit 1
}

info() {
  echo "$*"
}

# Load non-secret OAuth-related vars from .env when present.
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

SIWX_DOMAIN="${SIWX_DOMAIN:-localhost:3000}"
PORT="${PORT:-3000}"
GITHUB_REDIRECT_URI="${GITHUB_REDIRECT_URI:-}"

expected_callback_url() {
  local uri="${GITHUB_REDIRECT_URI//[[:space:]]/}"
  if [[ -n "$uri" ]]; then
    echo "$uri"
    return
  fi
  if [[ "$SIWX_DOMAIN" == *127.0.0.1* ]]; then
    echo "http://127.0.0.1:${PORT}/auth/callback"
  elif [[ "$SIWX_DOMAIN" == *localhost* ]]; then
    echo "http://localhost:${PORT}/auth/callback"
  else
    echo "https://${SIWX_DOMAIN}/auth/callback"
  fi
}

EXPECTED="$(expected_callback_url)"

info "Local GitHub OAuth check (${BASE})"
info ""
info "From .env (non-secret):"
info "  SIWX_DOMAIN=${SIWX_DOMAIN}"
info "  PORT=${PORT}"
if [[ -n "${GITHUB_REDIRECT_URI//[[:space:]]/}" ]]; then
  info "  GITHUB_REDIRECT_URI=${GITHUB_REDIRECT_URI}"
else
  info "  GITHUB_REDIRECT_URI=(unset — derived from SIWX_DOMAIN + PORT)"
fi
info ""
info "Expected callback URL (register this on your GitHub OAuth app):"
info "  ${EXPECTED}"
info ""

headers="$(mktemp)"
trap 'rm -f "$headers"' EXIT

if ! curl -sS -D "$headers" -o /dev/null "${BASE}/auth/github"; then
  fail "Server not reachable at ${BASE}. Start it with ./scripts/restart-dev.sh"
fi

status_line="$(head -1 "$headers")"
if ! grep -qiE '^HTTP/[0-9.]+ 30[1278] ' "$headers"; then
  fail "GET /auth/github did not redirect (got: ${status_line}). Is the app running with SSR?"
fi

location="$(awk -F': ' 'tolower($1)=="location" {print substr($0, index($0,$2)); exit}' "$headers" | tr -d '\r')"
if [[ -z "$location" ]]; then
  fail "GET /auth/github redirect missing Location header"
fi

ACTUAL="$(/usr/bin/python3 -c "
from urllib.parse import urlparse, parse_qs, unquote
import sys
loc = sys.argv[1]
qs = parse_qs(urlparse(loc).query)
uri = qs.get('redirect_uri', [''])[0]
print(unquote(uri))
" "$location")"

if [[ -z "$ACTUAL" ]]; then
  fail "Could not parse redirect_uri from Location header"
fi

info "Server /auth/github redirect_uri:"
info "  ${ACTUAL}"
info ""

if [[ "$ACTUAL" != "$EXPECTED" ]]; then
  fail "$(cat <<EOF
redirect_uri mismatch.

  Expected (from .env): ${EXPECTED}
  Server sent:          ${ACTUAL}

Fix:
  1. Restart the dev server after changing .env (./scripts/restart-dev.sh).
  2. Or set GITHUB_REDIRECT_URI to match what the server should send.
  3. Register the expected URL on GitHub:
     Settings → Developer settings → OAuth Apps → your local app
     → Authorization callback URL = ${EXPECTED}
EOF
)"
fi

info "AUTH CHECK OK: server redirect_uri matches .env expectation."
info ""
info "If GitHub still shows \"redirect_uri is not associated with this application\":"
info "  1. Open https://github.com/settings/developers"
info "  2. Select the OAuth app whose Client ID is in your .env GITHUB_CLIENT_ID"
info "  3. Set Authorization callback URL to exactly:"
info "       ${EXPECTED}"
info "  4. Save, wait a few seconds, then try Sign in with GitHub again."