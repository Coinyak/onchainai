#!/usr/bin/env bash
# Self-contained checks for split smoke-test scripts (no live network).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

fail() {
  echo "SMOKE TEST HARNESS FAIL: $*" >&2
  exit 1
}

pass() {
  echo "SMOKE TEST HARNESS PASS: $*"
}

assert_contains() {
  local needle="$1"
  local file="$2"
  grep -Fq "$needle" "$file" || fail "expected ${file} to contain: ${needle}"
}

assert_not_contains() {
  local needle="$1"
  local file="$2"
  if grep -Fq "$needle" "$file"; then
    fail "expected ${file} NOT to contain: ${needle}"
  fi
}

for script in smoke-test.sh smoke-test-api.sh smoke-test-frontend.sh smoke-test-common.sh; do
  [[ -f "${ROOT}/scripts/${script}" ]] || fail "missing scripts/${script}"
done

# Leptos smoke must stay for CI/local release binary.
assert_contains 'Crypto tool coverage' "${ROOT}/scripts/smoke-test.sh"
assert_contains 'Sign in to save your stack' "${ROOT}/scripts/smoke-test.sh"
assert_contains 'wallet-sign-in' "${ROOT}/scripts/smoke-test.sh"

# API smoke targets Railway contract.
assert_contains 'get_dashboard_snapshot' "${ROOT}/scripts/smoke-test-api.sh"
assert_contains '/api/v2/blueprints' "${ROOT}/scripts/smoke-test-api.sh"
assert_contains 'Vercel frontend' "${ROOT}/scripts/smoke-test-api.sh"

# Frontend smoke targets Next.js split deploy.
assert_contains 'connect-page' "${ROOT}/scripts/smoke-test-frontend.sh"
assert_contains 'wallet-sign-in-link' "${ROOT}/scripts/smoke-test-frontend.sh"
assert_contains '_next/static' "${ROOT}/scripts/smoke-test-frontend.sh"
assert_not_contains 'sidebar-brand' "${ROOT}/scripts/smoke-test-frontend.sh"

# Deploy gate must not curl-smoke Vercel with the Leptos script.
assert_contains 'smoke-test-api.sh' "${ROOT}/scripts/deploy-railway.sh"
assert_not_contains './scripts/smoke-test.sh "${PROD_URL}"' "${ROOT}/scripts/deploy-railway.sh"

# Post-deploy verify runs both split smokes.
assert_contains 'smoke-test-frontend.sh' "${ROOT}/scripts/post-deploy-verify.sh"
assert_contains 'smoke-test-api.sh' "${ROOT}/scripts/post-deploy-verify.sh"

# Stack detection helper.
# shellcheck source=scripts/smoke-test-common.sh
source "${ROOT}/scripts/smoke-test-common.sh"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

stub_curl="${tmpdir}/curl"
cat > "$stub_curl" <<'EOF'
#!/usr/bin/env bash
out=""
write_code=""
while (($# > 0)); do
  case "$1" in
    -o)
      out="$2"
      shift 2
      ;;
    -w)
      write_code="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
if [[ -n "$out" && -n "${SMOKE_STUB_STACK:-}" ]]; then
  printf '%s' "${SMOKE_STUB_STACK}" >"$out"
fi
if [[ -n "$write_code" ]]; then
  if [[ "$write_code" == *http_code* ]]; then
    printf '200'
  else
    printf '%s' "$write_code"
  fi
  exit 0
fi
printf '000'
exit 1
EOF
chmod +x "$stub_curl"

export PATH="${tmpdir}:${PATH}"

export SMOKE_STUB_STACK='<html><script src="/_next/static/chunks/app.js"></script></html>'
[[ "$(smoke_detect_stack "http://stub")" == "frontend" ]] || fail "detect frontend"

export SMOKE_STUB_STACK='<html><aside class="sidebar-brand"></aside><script src="/pkg/onchainai.js"></script></html>'
[[ "$(smoke_detect_stack "http://stub")" == "leptos" ]] || fail "detect leptos"

multiline=$'line1\nsite-top-nav\nline3'
smoke_body_has "$multiline" 'site-top-nav' || fail "smoke_body_has multiline"
! smoke_body_has "$multiline" 'missing-marker' || fail "smoke_body_has negative"

pass "split smoke scripts and deploy wiring"