#!/usr/bin/env bash
# Wave 2 production verification — W4 L4 probe history, W3 Admin UI + attribution, W8 Stale Trust Badge + K2 gate.
#
# W4 (L4) — optional SQL when DATABASE_URL is set:
#   L4-1  x402_probe_history row counts (scheduled cron writes tool_id + status)
#   L4-2  recent probe sample (status, probed_at; no secrets)
#   L4-3  quarantined tool count (14-day consecutive failure auto-quarantine)
#   L4-4  cron-eligible x402 tools + x402_last_checked_at coverage
#   L4-5  SKIP_CRAWLER does not gate x402 verify (X402_VERIFY_DISABLED only; static grep)
#
# W3 (REST + Vercel bundles):
#   W3-1  GET /api/v2/settings returns public site-settings shape (x402 + MCP premium fields)
#   W3-2  GET /api/v2/admin/settings requires auth (403 without session; not /api/v2/site-settings)
#   W3-3  x402 catalog lists approved tools (exa + token-price minimum) via tools/list
#   W3-4  /x402 hub shell + client bundle markers (x402-hub-live-list)
#   W3-5  Admin settings bundle markers (allow-x402-registration, x402-site-settings, default-referral-bps)
#   W3-6  Admin per-tool referral panel bundle (tool-referral-panel)
#   W3-7  Web install-guide attribution route deployed (POST /api/v2/tools/{slug}/attribution + bundle)
#
# W8 (POST /mcp):
#   W8-1  get_tool_detail includes trust_probe (last_probe_at, live, stale, skip_cost, k2_conversion_reason)
#   W8-2  compare_tools includes trust_probe on each x402-priced tool
#   W8-3  check_endpoint_health returns HTTP 402 + PAYMENT-REQUIRED + accepts[] (no wallet)
#
# Probe Receipt (paid K2 settle) is owner-only — not run here:
#   EVM_PRIVATE_KEY=0x... node scripts/x402-premium-e2e.mjs [slug] [api_base]
#
# Environment:
#   ONCHAINAI_FRONTEND_URL / PROD_URL          — Vercel origin (default: https://www.onchain-ai.xyz)
#   ONCHAINAI_MCP_URL / RAILWAY_API_URL        — API origin (default: onchainai-production.up.railway.app)
#   DATABASE_URL                               — optional prod DB for L4 SQL checks (.env auto-sourced)
#   ONCHAINAI_L4_MIN_PROBE_ROWS                — minimum x402_probe_history rows (default: 1)
#   ONCHAINAI_L4_MIN_CHECKED_TOOLS             — minimum cron-eligible tools with x402_last_checked_at (default: 1)
#   ONCHAINAI_W3_ATTRIBUTION_SLUG              — slug for attribution smoke (default: exa)
#   ONCHAINAI_W3_REQUIRED_X402_SLUGS         — comma-separated required x402 slugs (default: exa,token-price)
#   ONCHAINAI_W8_DETAIL_SLUG                   — x402 tool for get_tool_detail (default: goldrush-x402)
#   ONCHAINAI_W8_COMPARE_SLUGS                 — comma-separated x402 slugs for compare_tools (default: goldrush-x402,exa)
#   ONCHAINAI_W8_PROBE_SLUG                    — slug for check_endpoint_health 402 gate (default: goldrush-x402)
#
# Usage:
#   ./scripts/wave2-prod-verify.sh
#   ./scripts/wave2-prod-verify.sh --w3-only
#   ./scripts/wave2-prod-verify.sh --w8-only
#   ./scripts/wave2-prod-verify.sh --l4-only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/smoke-test-common.sh
source "${ROOT}/scripts/smoke-test-common.sh"

FRONTEND_URL="${ONCHAINAI_FRONTEND_URL:-${PROD_URL:-https://www.onchain-ai.xyz}}"
FRONTEND_URL="${FRONTEND_URL%/}"
MCP_URL="${ONCHAINAI_MCP_URL:-${RAILWAY_API_URL:-https://onchainai-production.up.railway.app}}"
MCP_URL="${MCP_URL%/}"
API_URL="${API_URL:-$MCP_URL}"
ATTRIBUTION_SLUG="${ONCHAINAI_W3_ATTRIBUTION_SLUG:-exa}"
REQUIRED_X402_SLUGS="${ONCHAINAI_W3_REQUIRED_X402_SLUGS:-exa,token-price}"
DETAIL_SLUG="${ONCHAINAI_W8_DETAIL_SLUG:-goldrush-x402}"
COMPARE_SLUGS="${ONCHAINAI_W8_COMPARE_SLUGS:-goldrush-x402,exa}"
PROBE_SLUG="${ONCHAINAI_W8_PROBE_SLUG:-goldrush-x402}"
L4_MIN_PROBE_ROWS="${ONCHAINAI_L4_MIN_PROBE_ROWS:-1}"
L4_MIN_CHECKED_TOOLS="${ONCHAINAI_L4_MIN_CHECKED_TOOLS:-1}"

if [[ -f "${ROOT}/.env" && -z "${DATABASE_URL:-}" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "${ROOT}/.env"
  set +a
fi

RUN_W3=true
RUN_W8=true
RUN_L4=true
for arg in "$@"; do
  case "$arg" in
    --w3-only) RUN_W8=false; RUN_L4=false ;;
    --w8-only) RUN_W3=false; RUN_L4=false ;;
    --l4-only) RUN_W3=false; RUN_W8=false ;;
    -*) echo "Unknown flag: $arg" >&2; exit 1 ;;
  esac
done

W8_TRUST_PROBE_KEYS=(
  last_probe_at
  live
  stale
  skip_cost
  k2_conversion_reason
)

w3_fail() {
  echo "W3 VERIFY FAIL: $*" >&2
  exit 1
}

w8_fail() {
  echo "W8 VERIFY FAIL: $*" >&2
  exit 1
}

l4_fail() {
  echo "L4 VERIFY FAIL: $*" >&2
  exit 1
}

verify_l4_skip_crawler_decoupled() {
  echo "--- L4-5: SKIP_CRAWLER decoupled from x402 verify scheduler ---"
  if ! grep -q 'skip_x402_verify = std::env::var("X402_VERIFY_DISABLED")' "${ROOT}/src/lib.rs"; then
    l4_fail "src/lib.rs missing X402_VERIFY_DISABLED gate (x402 verify may still depend on SKIP_CRAWLER)"
  fi
  if grep -q 'skip_crawler.*x402_verify\|SKIP_CRAWLER.*x402' "${ROOT}/src/lib.rs"; then
    l4_fail "src/lib.rs still couples SKIP_CRAWLER to x402 verify"
  fi
  if ! grep -q 'DEFAULT_X402_VERIFY_CRON' "${ROOT}/src/server/x402_verify.rs"; then
    l4_fail "src/server/x402_verify.rs missing DEFAULT_X402_VERIFY_CRON"
  fi
  if ! grep -q 'maybe_auto_quarantine_l4' "${ROOT}/src/server/x402_verify.rs"; then
    l4_fail "src/server/x402_verify.rs missing L4 auto-quarantine"
  fi
  echo "L4-5 PASS: x402 verify uses X402_VERIFY_DISABLED; cron default 03:00 UTC; L4 quarantine present"
}

verify_l4_db() {
  echo "--- L4 SQL: x402_probe_history + quarantine (DATABASE_URL set) ---"
  python3 - <<'PY' || l4_fail "L4 SQL checks failed"
import os, sys

min_rows = int(os.environ.get("ONCHAINAI_L4_MIN_PROBE_ROWS", "1"))
min_checked = int(os.environ.get("ONCHAINAI_L4_MIN_CHECKED_TOOLS", "1"))

try:
    import psycopg2
except ImportError:
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "psycopg2-binary"])
    import psycopg2

public_where = """
approval_status = 'approved'
AND relevance_status = 'accepted'
AND NOT (crypto_relevance_score = 0
  AND 'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons))
AND install_risk_level <> 'critical'
AND quarantined_at IS NULL
"""

conn = psycopg2.connect(os.environ["DATABASE_URL"])
cur = conn.cursor()

cur.execute("SELECT COUNT(*) FROM x402_probe_history")
total_rows = cur.fetchone()[0]
print(f"L4-1 probe_history_total={total_rows}")

cur.execute(
    "SELECT COUNT(*) FROM x402_probe_history WHERE tool_id IS NOT NULL"
)
scheduled_rows = cur.fetchone()[0]
print(f"L4-1 probe_history_with_tool_id={scheduled_rows}")

cur.execute(
    """
    SELECT status, COUNT(*) FROM x402_probe_history
    GROUP BY status ORDER BY 2 DESC
    """
)
print("L4-1 status_counts=" + str(cur.fetchall()))

cur.execute(
    """
    SELECT COALESCE(tool_id::text, 'null'), status,
           left(endpoint_url, 48), latency_ms, probed_at
    FROM x402_probe_history
    ORDER BY probed_at DESC
    LIMIT 5
    """
)
print("L4-2 recent_probes:")
for row in cur.fetchall():
    print(" ", row)

cur.execute("SELECT COUNT(*) FROM tools WHERE quarantined_at IS NOT NULL")
quarantined = cur.fetchone()[0]
print(f"L4-3 quarantined_tools={quarantined}")

cur.execute(
    f"""
    SELECT COUNT(*) FROM tools
    WHERE pricing = 'x402'
      AND x402_endpoint IS NOT NULL
      AND trim(x402_endpoint) <> ''
      AND {public_where}
    """
)
eligible = cur.fetchone()[0]
cur.execute(
    f"""
    SELECT COUNT(*) FROM tools
    WHERE pricing = 'x402'
      AND x402_endpoint IS NOT NULL
      AND trim(x402_endpoint) <> ''
      AND {public_where}
      AND x402_last_checked_at IS NOT NULL
    """
)
checked = cur.fetchone()[0]
print(f"L4-4 cron_eligible_x402_tools={eligible} checked_tools={checked}")

cur.close()
conn.close()

failures = []
if total_rows < min_rows:
    failures.append(f"probe_history_total {total_rows} < {min_rows}")
if scheduled_rows < 1:
    failures.append("no probe_history rows with tool_id (scheduled cron not writing yet)")
if checked < min_checked:
    failures.append(f"checked_tools {checked} < {min_checked} (03:00 UTC cron may not have run)")

if failures:
    for msg in failures:
        print(f"L4 VERIFY FAIL: {msg}", file=sys.stderr)
    sys.exit(1)

print("L4 SQL PASS")
PY
}

# Search linked Next.js chunks for a substring (client-only UI markers).
frontend_page_bundle_has() {
  local page_body="$1"
  local needle="$2"
  local asset
  while IFS= read -r asset; do
    [[ -z "$asset" ]] && continue
    local asset_body
    asset_body="$(curl -sS "${FRONTEND_URL}/${asset}" 2>/dev/null)" || continue
    if smoke_body_has "$asset_body" "$needle"; then
      return 0
    fi
  done < <(printf '%s' "$page_body" | grep -oE '_next/static/chunks/[^" ]+\.(js|css)' | sort -u)
  return 1
}

_MCP_HTTP_CODE=""
_MCP_BODY=""
_MCP_HEADERS=""
mcp_tools_call() {
  local tool_name="$1"
  local arguments_json="$2"
  _MCP_BODY="$(mktemp)"
  _MCP_HEADERS="$(mktemp)"
  local payload
  payload="$(printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"%s","arguments":%s}}' \
    "$tool_name" "$arguments_json")"
  _MCP_HTTP_CODE="$(curl -sS -D "$_MCP_HEADERS" -o "$_MCP_BODY" -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "${MCP_URL}/mcp")" || w8_fail "POST /mcp tools/call ${tool_name} curl failed"
}

mcp_cleanup() {
  rm -f "$_MCP_BODY" "$_MCP_HEADERS"
}

mcp_result_text() {
  python3 - "$_MCP_BODY" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
if "error" in data:
    print(json.dumps(data["error"]))
    sys.exit(2)
text = data["result"]["content"][0]["text"]
print(text)
PY
}

assert_trust_probe_json() {
  local label="$1"
  local json_blob="$2"
  printf '%s' "$json_blob" | python3 -c "
import json, sys
label = sys.argv[1]
required = sys.argv[2:]
obj = json.load(sys.stdin)
tp = obj.get('trust_probe')
if tp is None:
    print(f'{label}: trust_probe missing', file=sys.stderr)
    sys.exit(1)
missing = [k for k in required if k not in tp]
if missing:
    print(f'{label}: missing keys {missing}', file=sys.stderr)
    sys.exit(1)
if not isinstance(tp.get('skip_cost'), dict):
    print(f'{label}: skip_cost must be object', file=sys.stderr)
    sys.exit(1)
for key in ('probe_cost_usd', 'estimated_dead_call_loss_usd', 'message'):
    if key not in tp['skip_cost']:
        print(f'{label}: skip_cost missing {key}', file=sys.stderr)
        sys.exit(1)
print(f'{label}: trust_probe ok')
" "$label" "${W8_TRUST_PROBE_KEYS[@]}" || w8_fail "$label trust_probe validation failed"
}

compare_slugs_json_array() {
  local IFS=,
  local -a parts=()
  local slug
  for slug in $COMPARE_SLUGS; do
    slug="${slug//[[:space:]]/}"
    [[ -n "$slug" ]] || continue
    parts+=("\"${slug}\"")
  done
  if ((${#parts[@]} < 2)); then
    w8_fail "ONCHAINAI_W8_COMPARE_SLUGS must list at least two slugs (got: ${COMPARE_SLUGS})"
  fi
  local joined
  joined="$(IFS=,; echo "${parts[*]}")"
  printf '{"slugs":[%s]}' "$joined"
}

if [[ "$RUN_W3" == "true" ]]; then
  echo "=== Wave 2 prod verify (W3): frontend=${FRONTEND_URL} api=${API_URL} ==="

  echo "--- W3-1: public site settings shape (GET /api/v2/settings) ---"
  settings_body="$(mktemp)"
  settings_code="$(curl -sS -o "$settings_body" -w "%{http_code}" "${API_URL}/api/v2/settings")" \
    || w3_fail "GET /api/v2/settings curl failed"
  [[ "$settings_code" == "200" ]] || w3_fail "GET /api/v2/settings expected 200, got ${settings_code}"
  for key in allow_x402_registration mcp_premium_enabled mcp_premium_network site_name; do
    grep -q "\"${key}\"" "$settings_body" || w3_fail "GET /api/v2/settings missing ${key}"
  done
  # Public sanitize: operator payout fields must not leak.
  if grep -q '"default_referral_bps":' "$settings_body"; then
    if ! grep -q '"default_referral_bps":null' "$settings_body"; then
      rm -f "$settings_body"
      w3_fail "GET /api/v2/settings leaked default_referral_bps (expected null)"
    fi
  fi
  rm -f "$settings_body"

  echo "--- W3-2: admin site settings auth gate (GET /api/v2/admin/settings) ---"
  admin_code="$(curl -sS -o /dev/null -w "%{http_code}" "${API_URL}/api/v2/admin/settings")" \
    || w3_fail "GET /api/v2/admin/settings curl failed"
  [[ "$admin_code" == "401" || "$admin_code" == "403" ]] \
    || w3_fail "GET /api/v2/admin/settings expected 401/403 without auth, got ${admin_code}"
  legacy_code="$(curl -sS -o /dev/null -w "%{http_code}" "${API_URL}/api/v2/site-settings")" \
    || w3_fail "GET /api/v2/site-settings curl failed"
  [[ "$legacy_code" == "404" ]] \
    || w3_fail "GET /api/v2/site-settings expected 404 (use /api/v2/settings or /api/v2/admin/settings), got ${legacy_code}"

  echo "--- W3-3: x402 catalog includes required slugs (${REQUIRED_X402_SLUGS}) ---"
  x402_list_body="$(mktemp)"
  x402_list_code="$(curl -sS -o "$x402_list_body" -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -d '{"sort":"new","offset":0,"limit":50,"filters":{"tool_type":["x402"]}}' \
    "${API_URL}/api/v2/tools/list")" || w3_fail "POST /api/v2/tools/list x402 curl failed"
  [[ "$x402_list_code" == "200" ]] || w3_fail "POST /api/v2/tools/list x402 returned ${x402_list_code}"
  printf '%s' "$(cat "$x402_list_body")" | python3 -c "
import json, sys
required = [s.strip() for s in sys.argv[1].split(',') if s.strip()]
items = json.load(sys.stdin)
slugs = {t.get('slug') for t in items if isinstance(t, dict)}
missing = [s for s in required if s not in slugs]
if missing:
    raise SystemExit(f'missing x402 slugs: {missing} (have {sorted(slugs)})')
print(f'x402 tools ok: {sorted(slugs)}')
" "$REQUIRED_X402_SLUGS" || w3_fail "x402 catalog missing required slugs"
  rm -f "$x402_list_body"

  echo "--- W3-4: /x402 hub page markers ---"
  x402_page_body="$(curl -sS -L "${FRONTEND_URL}/x402")" || w3_fail "GET /x402 curl failed"
  if smoke_body_has "$x402_page_body" 'data-testid="x402-hub-live-list"'; then
    echo "/x402 SSR shell includes x402-hub-live-list"
  elif frontend_page_bundle_has "$x402_page_body" 'x402-hub-live-list'; then
    echo "/x402 client bundle includes x402-hub-live-list"
  else
    w3_fail "GET /x402 missing x402-hub-live-list marker"
  fi
  smoke_body_has "$x402_page_body" 'data-testid="x402-hub-list-cta"' \
    || w3_fail "GET /x402 missing x402-hub-list-cta"

  echo "--- W3-5: admin settings UI bundle markers ---"
  # Do not follow redirect to /login — the 307 shell still preloads the settings page chunk.
  admin_settings_body="$(curl -sS "${FRONTEND_URL}/admin/settings")" || w3_fail "GET /admin/settings curl failed"
  for needle in allow-x402-registration x402-site-settings default-referral-bps mcp-premium-enabled; do
    frontend_page_bundle_has "$admin_settings_body" "$needle" \
      || w3_fail "GET /admin/settings bundles missing ${needle}"
    echo "admin settings bundle ok: ${needle}"
  done

  echo "--- W3-6: admin per-tool referral panel bundle ---"
  admin_tools_body="$(curl -sS "${FRONTEND_URL}/admin/tools")" || w3_fail "GET /admin/tools curl failed"
  frontend_page_bundle_has "$admin_tools_body" 'tool-referral-panel' \
    || w3_fail "GET /admin/tools bundles missing tool-referral-panel"

  echo "--- W3-7: web install-guide attribution (POST + bundle) ---"
  attr_body="$(mktemp)"
  attr_code="$(curl -sS -o "$attr_body" -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -d "{\"platform\":\"cursor\",\"attribution_session\":\"wave2-w3-verify\"}" \
    "${API_URL}/api/v2/tools/${ATTRIBUTION_SLUG}/attribution")" \
    || w3_fail "POST /api/v2/tools/${ATTRIBUTION_SLUG}/attribution curl failed"
  [[ "$attr_code" == "200" ]] || w3_fail "POST attribution expected 200, got ${attr_code}"
  grep -q '"ok"' "$attr_body" || w3_fail "POST attribution missing ok field"
  grep -q '"recorded"' "$attr_body" || w3_fail "POST attribution missing recorded field"
  rm -f "$attr_body"
  tool_page_body="$(curl -sS -L "${FRONTEND_URL}/tools/${ATTRIBUTION_SLUG}")" \
    || w3_fail "GET /tools/${ATTRIBUTION_SLUG} curl failed"
  frontend_page_bundle_has "$tool_page_body" '/attribution' \
    || w3_fail "GET /tools/${ATTRIBUTION_SLUG} bundles missing /attribution path"
  frontend_page_bundle_has "$tool_page_body" 'install-guide-panel' \
    || w3_fail "GET /tools/${ATTRIBUTION_SLUG} bundles missing install-guide-panel"

  echo "W3 VERIFY PASS frontend=${FRONTEND_URL} api=${API_URL}"
fi

if [[ "$RUN_L4" == "true" ]]; then
  echo "=== Wave 2 prod verify (W4 L4): ${MCP_URL} ==="
  verify_l4_skip_crawler_decoupled
  if [[ -n "${DATABASE_URL:-}" ]]; then
    export ONCHAINAI_L4_MIN_PROBE_ROWS="${L4_MIN_PROBE_ROWS}"
    export ONCHAINAI_L4_MIN_CHECKED_TOOLS="${L4_MIN_CHECKED_TOOLS}"
    verify_l4_db
  else
    echo "--- L4 SQL: skipped (DATABASE_URL unset) ---"
  fi
  if command -v railway >/dev/null 2>&1; then
    echo "--- L4 deploy log hint (railway logs, last 500 lines) ---"
    railway logs --lines 500 2>/dev/null | grep -E 'x402 verify scheduler spawned|crawler scheduler skipped|scheduled job: x402 verification|x402 scheduled verify' \
      | tail -5 || echo "(no matching Railway log lines — cron runs 03:00 UTC daily)"
  fi
fi

if [[ "$RUN_W8" == "true" ]]; then
  echo "=== Wave 2 prod verify (W8): ${MCP_URL} ==="

  echo "--- W8-1: get_tool_detail trust_probe (${DETAIL_SLUG}) ---"
mcp_tools_call "get_tool_detail" "{\"slug\":\"${DETAIL_SLUG}\"}"
if [[ "$_MCP_HTTP_CODE" != "200" ]]; then
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  w8_fail "get_tool_detail expected HTTP 200, got ${_MCP_HTTP_CODE}"
fi
detail_text="$(mcp_result_text)" || {
  mcp_cleanup
  w8_fail "get_tool_detail JSON-RPC error"
}
assert_trust_probe_json "get_tool_detail" "$detail_text"
mcp_cleanup

echo "--- W8-2: compare_tools trust_probe (${COMPARE_SLUGS}) ---"
compare_args="$(compare_slugs_json_array)"
mcp_tools_call "compare_tools" "$compare_args"
if [[ "$_MCP_HTTP_CODE" != "200" ]]; then
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  w8_fail "compare_tools expected HTTP 200, got ${_MCP_HTTP_CODE}"
fi
compare_text="$(mcp_result_text)" || {
  mcp_cleanup
  w8_fail "compare_tools JSON-RPC error"
}
printf '%s' "$compare_text" | python3 -c "
import json, sys
required = {'last_probe_at', 'live', 'stale', 'skip_cost', 'k2_conversion_reason'}
rows = json.load(sys.stdin)
if len(rows) < 2:
    raise SystemExit('compare_tools returned fewer than 2 rows')
for row in rows:
    slug = row['tool']['slug']
    tp = row.get('trust_probe')
    if tp is None:
        raise SystemExit(f'{slug}: trust_probe missing')
    missing = required - set(tp)
    if missing:
        raise SystemExit(f'{slug}: missing {sorted(missing)}')
    sc = tp.get('skip_cost')
    if not isinstance(sc, dict) or not {'probe_cost_usd', 'estimated_dead_call_loss_usd', 'message'} <= set(sc):
        raise SystemExit(f'{slug}: skip_cost incomplete')
    print(f'{slug}: trust_probe ok')
" || w8_fail "compare_tools trust_probe validation failed"
mcp_cleanup

echo "--- W8-3: check_endpoint_health 402 gate (${PROBE_SLUG}, no wallet) ---"
mcp_tools_call "check_endpoint_health" "{\"slug\":\"${PROBE_SLUG}\"}"
health_code="$_MCP_HTTP_CODE"
payment_header=""
if [[ -f "$_MCP_HEADERS" ]]; then
  payment_header="$(grep -i '^payment-required:' "$_MCP_HEADERS" | head -1 | sed 's/^[^:]*:[[:space:]]*//' || true)"
fi

if [[ "$health_code" == "503" ]]; then
  head -20 "$_MCP_BODY" >&2
  mcp_cleanup
  w8_fail "check_endpoint_health returned 503 — K2 x402 not configured"
fi
if [[ "$health_code" != "402" ]]; then
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  w8_fail "check_endpoint_health expected HTTP 402, got ${health_code}"
fi
if [[ -z "$payment_header" ]]; then
  head -30 "$_MCP_HEADERS" >&2
  mcp_cleanup
  w8_fail "check_endpoint_health missing PAYMENT-REQUIRED header"
fi
if ! grep -q '"accepts"' "$_MCP_BODY"; then
  head -40 "$_MCP_BODY" >&2
  mcp_cleanup
  w8_fail "check_endpoint_health 402 body missing accepts[]"
fi
if ! grep -q '"x402Version"' "$_MCP_BODY"; then
  mcp_cleanup
  w8_fail "check_endpoint_health 402 body missing x402Version"
fi
mcp_cleanup

echo "--- W8 note: probe_receipt requires paid settle (owner-only) ---"
echo "  Run: EVM_PRIVATE_KEY=0x... node scripts/x402-premium-e2e.mjs ${PROBE_SLUG} ${API_URL}"

  echo "W8 VERIFY PASS ${MCP_URL} (detail=${DETAIL_SLUG}, compare=${COMPARE_SLUGS}, probe=${PROBE_SLUG})"
fi

if [[ "$RUN_W3" == "true" || "$RUN_L4" == "true" || "$RUN_W8" == "true" ]]; then
  echo "WAVE2 PROD VERIFY PASS (w3=${RUN_W3} l4=${RUN_L4} w8=${RUN_W8})"
fi