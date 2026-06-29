#!/usr/bin/env bash
# Self-contained tests for scripts/local-doctor.sh using fake curl/lsof/ps.
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOCTOR_SOURCE="${SOURCE_ROOT}/scripts/local-doctor.sh"

fail() {
  echo "LOCAL DOCTOR TEST FAIL: $*" >&2
  exit 1
}

pass() {
  echo "LOCAL DOCTOR TEST PASS: $*"
}

assert_exit_capture() {
  local expected="$1"
  local output="$2"
  shift 2
  set +e
  "$@" >"$output" 2>&1
  local status=$?
  set -e
  if [[ "$status" != "$expected" ]]; then
    cat "$output" >&2
    fail "expected exit ${expected}, got ${status}: $*"
  fi
}

assert_contains() {
  local needle="$1"
  local file="$2"
  grep -Fq "$needle" "$file" || {
    cat "$file" >&2
    fail "expected output to contain: ${needle}"
  }
}

[[ -f "$DOCTOR_SOURCE" ]] || fail "missing ${DOCTOR_SOURCE}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/repo/scripts" "$tmpdir/bin" "$tmpdir/repo/target"
cp "$DOCTOR_SOURCE" "$tmpdir/repo/scripts/local-doctor.sh"
chmod +x "$tmpdir/repo/scripts/local-doctor.sh"

cat > "$tmpdir/repo/scripts/verify-bundle.sh" <<'EOF'
#!/usr/bin/env bash
if [[ "${FAKE_VERIFY_BUNDLE:-ok}" == "fail" ]]; then
  echo "bundle stale" >&2
  exit 1
fi
echo "bundle ok"
EOF
chmod +x "$tmpdir/repo/scripts/verify-bundle.sh"

cat > "$tmpdir/bin/lsof" <<'EOF'
#!/usr/bin/env bash
if [[ -n "${FAKE_LSOF_STATUS:-}" ]]; then
  exit "$FAKE_LSOF_STATUS"
fi
if [[ "${FAKE_REQUIRE_LISTEN:-0}" == "1" ]]; then
  case " $* " in
    *" -sTCP:LISTEN "*) ;;
    *) exit 2 ;;
  esac
fi
if [[ "${FAKE_LISTENER:-1}" == "1" ]]; then
  echo "${FAKE_LISTENER_PID:-4242}"
fi
EOF
chmod +x "$tmpdir/bin/lsof"

cat > "$tmpdir/bin/ps" <<'EOF'
#!/usr/bin/env bash
if [[ "${FAKE_PS_ALIVE:-1}" == "1" ]]; then
  exit 0
fi
exit 1
EOF
chmod +x "$tmpdir/bin/ps"

cat > "$tmpdir/bin/curl" <<'EOF'
#!/usr/bin/env bash
if [[ "${FAKE_REQUIRE_HEAD:-0}" == "1" ]]; then
  case " $* " in
    *" -I "*|*" --head "*) ;;
    *) exit 2 ;;
  esac
fi
url=""
for arg do
  url="$arg"
done
path="${url#*://}"
path="/${path#*/}"
path="${path%%\?*}"

case "$path" in
  /)
    cache="${FAKE_HTML_CACHE:-private, no-cache, max-age=0, must-revalidate}"
    code="${FAKE_HTML_CODE:-200}"
    ;;
  /pkg/onchainai.js|/pkg/onchainai.wasm|/pkg/onchainai.css)
    cache="${FAKE_PKG_CACHE:-no-store}"
    code="${FAKE_PKG_CODE:-200}"
    ;;
  *)
    cache="${FAKE_OTHER_CACHE:-no-store}"
    code="${FAKE_OTHER_CODE:-200}"
    ;;
esac

printf 'HTTP/1.1 %s OK\r\n' "$code"
printf 'cache-control: %s\r\n' "$cache"
printf '\r\n'
EOF
chmod +x "$tmpdir/bin/curl"

run_doctor() {
  (
    cd "$tmpdir/repo"
    PATH="$tmpdir/bin:$PATH" "$tmpdir/repo/scripts/local-doctor.sh" "$@"
  )
}

run_doctor_no_listener() {
  FAKE_LISTENER=0 FAKE_PS_ALIVE=0 run_doctor "$@"
}

run_doctor_bad_html_cache() {
  FAKE_HTML_CACHE="public, max-age=600" run_doctor "$@"
}

run_doctor_html_smaxage_only() {
  FAKE_HTML_CACHE="private, no-cache, s-maxage=0, must-revalidate" run_doctor "$@"
}

run_doctor_html_extension_max_age_only() {
  FAKE_HTML_CACHE="private, no-cache, x-max-age=0, must-revalidate" run_doctor "$@"
}

run_doctor_bad_pkg_cache() {
  FAKE_PKG_CACHE="public, max-age=31536000, immutable" run_doctor "$@"
}

run_doctor_verify_failure() {
  FAKE_VERIFY_BUNDLE=fail run_doctor "$@"
}

run_doctor_lsof_failure() {
  FAKE_LSOF_STATUS=127 run_doctor "$@"
}

run_doctor_require_listen_lsof() {
  FAKE_REQUIRE_LISTEN=1 run_doctor "$@"
}

run_doctor_require_head_curl() {
  FAKE_REQUIRE_HEAD=1 run_doctor "$@"
}

run_doctor_port_mismatch() {
  PORT=3001 run_doctor --base http://localhost:3000
}

echo "test-local-doctor: temp repo at ${tmpdir}"

out="$tmpdir/healthy.out"
assert_exit_capture 0 "$out" run_doctor
assert_contains "OK: local server/cache looks healthy for http://localhost:3000" "$out"
assert_contains "OK: / cache-control is private, no-cache, max-age=0, must-revalidate" "$out"
assert_contains "OK: /pkg/onchainai.js cache-control is no-store" "$out"
pass "healthy localhost server"

out="$tmpdir/no-listener.out"
assert_exit_capture 1 "$out" run_doctor_no_listener
assert_contains "FAIL: no process is listening on port 3000" "$out"
assert_contains "Next: start the local loop with ./scripts/dev-watch.sh" "$out"
pass "missing listener reports next command"

out="$tmpdir/stale-pid.out"
printf '9999\n' > "$tmpdir/repo/target/dev-server.pid"
assert_exit_capture 1 "$out" run_doctor_no_listener
assert_contains "WARN: target/dev-server.pid points to a dead process: 9999" "$out"
assert_contains "FAIL: no process is listening on port 3000" "$out"
rm -f "$tmpdir/repo/target/dev-server.pid"
pass "stale pid is called out"

out="$tmpdir/html-cache.out"
assert_exit_capture 1 "$out" run_doctor_bad_html_cache
assert_contains "FAIL: / cache-control is public, max-age=600" "$out"
assert_contains "Expected: private, no-cache, max-age=0, must-revalidate" "$out"
pass "bad HTML cache header fails"

out="$tmpdir/html-smaxage.out"
assert_exit_capture 1 "$out" run_doctor_html_smaxage_only
assert_contains "FAIL: / cache-control is private, no-cache, s-maxage=0, must-revalidate" "$out"
assert_contains "Expected: private, no-cache, max-age=0, must-revalidate" "$out"
pass "s-maxage does not satisfy max-age"

out="$tmpdir/html-extension-max-age.out"
assert_exit_capture 1 "$out" run_doctor_html_extension_max_age_only
assert_contains "FAIL: / cache-control is private, no-cache, x-max-age=0, must-revalidate" "$out"
assert_contains "Expected: private, no-cache, max-age=0, must-revalidate" "$out"
pass "extension max-age token does not satisfy max-age"

out="$tmpdir/pkg-cache.out"
assert_exit_capture 1 "$out" run_doctor_bad_pkg_cache
assert_contains "FAIL: /pkg/onchainai.js cache-control is public, max-age=31536000, immutable" "$out"
assert_contains "Expected local dev pkg assets to be no-store" "$out"
pass "bad local pkg cache header fails"

out="$tmpdir/loopback.out"
assert_exit_capture 0 "$out" run_doctor --base http://127.0.0.1:3000
assert_contains "WARN: use http://localhost:3000 for browser/auth checks when possible" "$out"
assert_contains "OK: local server/cache looks healthy for http://127.0.0.1:3000" "$out"
pass "127.0.0.1 guidance warns without failing"

out="$tmpdir/no-lsof.out"
assert_exit_capture 0 "$out" run_doctor_lsof_failure
assert_contains "WARN: lsof not available; using HTTP reachability checks instead" "$out"
assert_contains "OK: local server/cache looks healthy for http://localhost:3000" "$out"
pass "lsof failure falls back to HTTP checks"

out="$tmpdir/listen-lsof.out"
assert_exit_capture 0 "$out" run_doctor_require_listen_lsof
assert_contains "OK: port 3000 has listener pid(s): 4242" "$out"
pass "lsof listener check is restricted to LISTEN sockets"

out="$tmpdir/head-curl.out"
assert_exit_capture 0 "$out" run_doctor_require_head_curl
assert_contains "OK: /pkg/onchainai.wasm cache-control is no-store" "$out"
pass "header probes use HEAD requests"

out="$tmpdir/invalid-port.out"
assert_exit_capture 2 "$out" run_doctor --port abc
assert_contains "FAIL: port must be a number between 1 and 65535: abc" "$out"
assert_exit_capture 2 "$out" run_doctor --port 70000
assert_contains "FAIL: port must be a number between 1 and 65535: 70000" "$out"
pass "invalid ports are rejected"

out="$tmpdir/invalid-base.out"
assert_exit_capture 2 "$out" run_doctor --base https://example.com:3000
assert_contains "FAIL: --base must be a local URL" "$out"
assert_exit_capture 2 "$out" run_doctor --base http://localhost:3000/foo
assert_contains "FAIL: --base must not include a path" "$out"
assert_exit_capture 2 "$out" run_doctor --base http://localhost:
assert_contains "FAIL: --base port must not be empty: http://localhost:" "$out"
assert_exit_capture 2 "$out" run_doctor --base http://127.0.0.1:
assert_contains "FAIL: --base port must not be empty: http://127.0.0.1:" "$out"
assert_exit_capture 2 "$out" run_doctor --base 'http://[::1]:'
assert_contains "FAIL: --base port must not be empty: http://[::1]:" "$out"
pass "invalid base URLs are rejected"

out="$tmpdir/mismatch.out"
assert_exit_capture 2 "$out" run_doctor_port_mismatch
assert_contains "FAIL: --base port 3000 does not match explicit port 3001" "$out"
pass "base and explicit port mismatch is rejected"

out="$tmpdir/base-no-port.out"
assert_exit_capture 0 "$out" run_doctor --base http://localhost --port 3001
assert_contains "Local doctor: http://localhost:3001" "$out"
assert_contains "OK: local server/cache looks healthy for http://localhost:3001" "$out"
pass "base without port is normalized to selected port"

out="$tmpdir/verify.out"
assert_exit_capture 1 "$out" run_doctor_verify_failure
assert_contains "FAIL: ./scripts/verify-bundle.sh failed" "$out"
assert_contains "Next: run ./scripts/dev-watch.sh while editing, or ./scripts/restart-dev.sh --foreground after a release build." "$out"
pass "bundle verification failure reports coherent rebuild path"

echo "LOCAL DOCTOR TESTS PASS"
