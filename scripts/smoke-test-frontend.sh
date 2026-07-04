#!/usr/bin/env bash
# Curl smoke for the Vercel Next.js frontend (split deploy; API proxied separately).
#
# Usage:
#   ./scripts/smoke-test-frontend.sh
#   ./scripts/smoke-test-frontend.sh https://www.onchain-ai.xyz
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/smoke-test-common.sh
source "${ROOT}/scripts/smoke-test-common.sh"

BASE="${1:-https://www.onchain-ai.xyz}"
BASE="${BASE%/}"

check_get() {
  smoke_check_get "$BASE" "$1"
}

# Blueprint v2 share panel controls are client-only; assert via linked JS/CSS bundles.
smoke_page_bundle_has() {
  local page_body="$1"
  local needle="$2"
  local asset
  while IFS= read -r asset; do
    [[ -z "$asset" ]] && continue
    local asset_body
    asset_body="$(curl -sS "${BASE}/${asset}" 2>/dev/null)" || continue
    if smoke_body_has "$asset_body" "$needle"; then
      return 0
    fi
  done < <(printf '%s' "$page_body" | grep -oE '_next/static/chunks/[^" ]+\.(js|css)' | sort -u)
  return 1
}

home_body="$(check_get "/")"
smoke_body_has "$home_body" 'site-top-nav' || smoke_fail "GET / missing site-top-nav markup"
smoke_body_has "$home_body" 'auth-sign-in' || smoke_fail "GET / missing auth-sign-in markup"
smoke_body_has "$home_body" 'data-testid="top-nav-sign-in"' || smoke_fail "GET / missing top-nav Sign in control"
smoke_body_has "$home_body" '>Sign in<' || smoke_fail "GET / missing Sign in label"
smoke_body_has "$home_body" 'data-testid="profile-menu"' && smoke_fail "GET / unexpected profile-menu when signed out"
smoke_body_has "$home_body" 'category-grid' && smoke_fail "GET / unexpected category-grid markup"
smoke_body_has "$home_body" '_next/static' || smoke_fail "GET / missing Next.js bundle markers"

login_body="$(check_get "/login")"
smoke_body_has "$login_body" 'Continue with GitHub' || smoke_fail "GET /login missing GitHub sign-in option"
smoke_body_has "$login_body" 'data-testid="github-sign-in"' || smoke_fail "GET /login missing github-sign-in test id"
smoke_body_has "$login_body" 'rel="external"' || smoke_fail "GET /login missing rel=external on GitHub sign-in link"
smoke_body_has "$login_body" 'data-testid="wallet-sign-in"' || smoke_fail "GET /login missing wallet sign-in button"
smoke_body_has "$login_body" 'id="login-title"' || smoke_fail "GET /login missing login-title heading"

tools_body="$(check_get "/tools")"
smoke_body_has "$tools_body" 'tool-card' || smoke_fail "GET /tools missing tool-card markup"

check_get "/tools?function=bridge&type=mcp" >/dev/null
check_get "/connect" >/dev/null
connect_body="$(check_get "/connect")"
smoke_body_has "$connect_body" 'data-testid="connect-page"' || smoke_fail "GET /connect missing connect-page test id"

blueprints_body="$(check_get "/blueprints")"
smoke_body_has "$blueprints_body" 'data-testid="blueprint-list"' || smoke_fail "GET /blueprints missing blueprint-list test id"

blueprint_draft_body="$(check_get "/blueprints/draft")"
smoke_body_has "$blueprint_draft_body" 'data-testid="blueprint-canvas"' || smoke_fail "GET /blueprints/draft missing blueprint-canvas test id"
smoke_body_has "$blueprint_draft_body" 'data-testid="blueprint-share-dock"' || smoke_fail "GET /blueprints/draft missing blueprint-share-dock test id"
smoke_body_has "$blueprint_draft_body" 'blueprint-share-dock-fab' || smoke_fail "GET /blueprints/draft missing blueprint-share-dock FAB"
smoke_page_bundle_has "$blueprint_draft_body" 'blueprint-share-prompt-edit' \
  || smoke_fail "GET /blueprints/draft missing blueprint-share-prompt-edit bundle marker"
if ! smoke_page_bundle_has "$blueprint_draft_body" 'blueprint-copy-prompt'; then
  smoke_page_bundle_has "$blueprint_draft_body" 'blueprint-share-copy-btn' \
    || smoke_fail "GET /blueprints/draft missing blueprint-copy-prompt bundle marker"
fi

check_get "/compare" >/dev/null
check_get "/dashboard" >/dev/null
check_get "/toolkit" >/dev/null

toolkit_body="$(check_get "/toolkit")"
smoke_body_has "$toolkit_body" 'data-testid="toolkit-sign-in"' || smoke_fail "GET /toolkit missing Sign in control"
smoke_body_has "$toolkit_body" 'href="/login"' || smoke_fail "GET /toolkit missing /login sign-in link"

for chain_svg in bitcoin bob polygon; do
  svg_code="$(curl -sS -o /dev/null -w "%{http_code}" "${BASE}/chains/${chain_svg}.svg")" \
    || smoke_fail "GET /chains/${chain_svg}.svg curl failed"
  [[ "$svg_code" == "200" ]] || smoke_fail "GET /chains/${chain_svg}.svg returned ${svg_code}"
done

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || smoke_fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || smoke_fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || smoke_fail "POST /mcp missing serverInfo"
rm -f "$mcp_body"

list_body="$(mktemp)"
list_code="$(curl -sS -o "$list_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"sort":"hot","offset":0,"limit":1,"filters":{}}' \
  "${BASE}/api/v2/tools/list")" || smoke_fail "POST /api/v2/tools/list curl failed"
[[ "$list_code" == "200" ]] || smoke_fail "POST /api/v2/tools/list returned ${list_code}"
grep -q '"slug"' "$list_body" || smoke_fail "POST /api/v2/tools/list missing tool payload"
rm -f "$list_body"

echo "SMOKE FRONTEND PASS ${BASE}"