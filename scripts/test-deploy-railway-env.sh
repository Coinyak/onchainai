#!/usr/bin/env bash
# Self-contained tests for production env normalization in deploy-railway.sh.
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOY_SOURCE="${SOURCE_ROOT}/scripts/deploy-railway.sh"

fail() {
  echo "DEPLOY RAILWAY ENV TEST FAIL: $*" >&2
  exit 1
}

pass() {
  echo "DEPLOY RAILWAY ENV TEST PASS: $*"
}

assert_contains() {
  local needle="$1"
  local file="$2"
  grep -Fq "$needle" "$file" || {
    cat "$file" >&2
    fail "expected output to contain: ${needle}"
  }
}

[[ -f "$DEPLOY_SOURCE" ]] || fail "missing ${DEPLOY_SOURCE}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/repo/scripts" "$tmpdir/bin"
cp "$DEPLOY_SOURCE" "$tmpdir/repo/scripts/deploy-railway.sh"
chmod +x "$tmpdir/repo/scripts/deploy-railway.sh"

cat > "$tmpdir/bin/railway" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  whoami)
    echo "Test User"
    ;;
  status)
    exit 0
    ;;
  variable)
    shift
    [[ "${1:-}" == "set" ]] || exit 2
    shift
    printf '%s\n' "$@" >> "${FAKE_RAILWAY_ARGS:?}"
    ;;
  *)
    exit 2
    ;;
esac
EOF
chmod +x "$tmpdir/bin/railway"

cat > "$tmpdir/repo/.env" <<'EOF'
DATABASE_URL=postgres://example
SUPABASE_URL=https://example.supabase.co
SUPABASE_ANON_KEY=anon
SUPABASE_SERVICE_KEY=service
GITHUB_CLIENT_ID=github-client
GITHUB_CLIENT_SECRET=github-secret
GITHUB_API_TOKEN=github-api
GITHUB_REDIRECT_URI=http://localhost:3000/auth/callback
JWT_SECRET=jwt-secret
EOF

echo "test-deploy-railway-env: temp repo at ${tmpdir}"

out="$tmpdir/deploy.out"
args="$tmpdir/railway-args.out"
: > "$args"
(
  cd "$tmpdir/repo"
  FAKE_RAILWAY_ARGS="$args" PATH="$tmpdir/bin:$PATH" \
    ./scripts/deploy-railway.sh --vars-only
) >"$out" 2>&1

assert_contains "SIWX_DOMAIN=www.onchain-ai.xyz" "$args"
assert_contains "GITHUB_REDIRECT_URI=https://www.onchain-ai.xyz/auth/callback" "$args"
if grep -Fq "GITHUB_REDIRECT_URI=http://localhost:3000/auth/callback" "$args"; then
  cat "$args" >&2
  fail "production deploy leaked local GitHub callback"
fi

pass "production callback overrides local .env"

echo "DEPLOY RAILWAY ENV TESTS PASS"
