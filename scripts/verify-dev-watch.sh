#!/usr/bin/env bash
# E2E check: cargo leptos watch rebuilds WASM when a hydrate source changes.
#
# Usage:
#   ./scripts/verify-dev-watch.sh
#   ./scripts/verify-dev-watch.sh --check-only
#   ./scripts/verify-dev-watch.sh --port 3000 --timeout 420
#
# Requires: .env with DB, cargo-leptos, and a cold/warm toolchain. Slow on first run.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PORT="3000"
TIMEOUT_SEC=420
CHECK_ONLY=false
PROBE_FILE="src/components/sidebar.rs"
WASM="target/site/pkg/onchainai.wasm"
LOG_FILE=""

usage() {
  sed -n '2,10p' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="${2:-}"
      shift 2
      ;;
    --timeout)
      TIMEOUT_SEC="${2:-}"
      shift 2
      ;;
    --check-only)
      CHECK_ONLY=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ "$CHECK_ONLY" == "true" ]]; then
  missing=()
  [[ -f "$PROBE_FILE" ]] || missing+=("$PROBE_FILE")
  [[ -x scripts/dev-watch.sh ]] || missing+=("scripts/dev-watch.sh")
  [[ -x scripts/smoke-test.sh ]] || missing+=("scripts/smoke-test.sh")
  if (( ${#missing[@]} > 0 )); then
    echo "VERIFY DEV WATCH CHECK ONLY FAIL: missing or not executable: ${missing[*]}" >&2
    exit 1
  fi
  echo "VERIFY DEV WATCH CHECK ONLY PASS"
  echo "  probe:  ${PROBE_FILE}"
  echo "  scripts: dev-watch.sh, smoke-test.sh"
  exit 0
fi

if [[ ! -f .env ]]; then
  echo "VERIFY DEV WATCH FAIL: missing .env" >&2
  exit 1
fi

if [[ ! -f "$PROBE_FILE" ]]; then
  echo "VERIFY DEV WATCH FAIL: missing probe file ${PROBE_FILE}" >&2
  exit 1
fi

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

if [[ "$(uname -s)" == "Darwin" && "${RUSTFLAGS:-}" != *"symbol-mangling-version"* ]]; then
  export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C symbol-mangling-version=v0"
fi

export SKIP_CRAWLER="${SKIP_CRAWLER:-1}"
export ONCHAINAI_PKG_NO_CACHE="${ONCHAINAI_PKG_NO_CACHE:-1}"
export LEPTOS_SITE_ADDR="127.0.0.1:${PORT}"
export PORT

LOG_FILE="$(mktemp -t onchainai-dev-watch-verify.XXXXXX.log)"
PROBE_BACKUP=""
watch_pid=""

restore_probe_file() {
  if [[ -n "$PROBE_BACKUP" && -f "$PROBE_BACKUP" ]]; then
    cp "$PROBE_BACKUP" "$PROBE_FILE"
    rm -f "$PROBE_BACKUP"
    PROBE_BACKUP=""
  fi
}

cleanup() {
  if [[ -n "$watch_pid" ]] && kill -0 "$watch_pid" 2>/dev/null; then
    kill "$watch_pid" 2>/dev/null || true
    wait "$watch_pid" 2>/dev/null || true
  fi
  restore_probe_file
  if [[ -n "$LOG_FILE" && -f "$LOG_FILE" ]]; then
    rm -f "$LOG_FILE"
  fi
}
trap cleanup EXIT

pids="$(lsof -ti ":${PORT}" 2>/dev/null || true)"
if [[ -n "$pids" ]]; then
  # shellcheck disable=SC2086
  kill ${pids} 2>/dev/null || true
  sleep 1
fi

echo "Starting dev-watch for verify (port ${PORT}, timeout ${TIMEOUT_SEC}s)"
./scripts/dev-watch.sh >"$LOG_FILE" 2>&1 &
watch_pid=$!

base="http://127.0.0.1:${PORT}"
deadline=$((SECONDS + TIMEOUT_SEC))

wait_for_http() {
  while (( SECONDS < deadline )); do
    if curl -fsS -o /dev/null "${base}/" 2>/dev/null; then
      return 0
    fi
    if ! kill -0 "$watch_pid" 2>/dev/null; then
      echo "VERIFY DEV WATCH FAIL: dev-watch exited before server was ready" >&2
      tail -40 "$LOG_FILE" >&2 || true
      return 1
    fi
    sleep 2
  done
  echo "VERIFY DEV WATCH FAIL: server not ready within ${TIMEOUT_SEC}s" >&2
  tail -40 "$LOG_FILE" >&2 || true
  return 1
}

wait_for_http

if [[ ! -f "$WASM" ]]; then
  echo "VERIFY DEV WATCH FAIL: ${WASM} missing after dev-watch ready" >&2
  exit 1
fi

wasm_before="$(stat -f %m "$WASM" 2>/dev/null || stat -c %Y "$WASM")"

# Mtime-only touch does not always trigger cargo/leptos watch when file content
# is unchanged. Append a transient probe comment so the watch loop rebuilds WASM.
PROBE_BACKUP="$(mktemp -t onchainai-dev-watch-probe.XXXXXX.bak)"
cp "$PROBE_FILE" "$PROBE_BACKUP"
printf '\n// verify-dev-watch-probe %s\n' "$(date +%s)" >>"$PROBE_FILE"

echo "Waiting for WASM rebuild after editing ${PROBE_FILE}"
while (( SECONDS < deadline )); do
  wasm_after="$(stat -f %m "$WASM" 2>/dev/null || stat -c %Y "$WASM")"
  if [[ "$wasm_after" -gt "$wasm_before" ]]; then
    echo "WASM rebuilt (${wasm_before} -> ${wasm_after})"
    ./scripts/smoke-test.sh "$base" >/dev/null
    restore_probe_file
    echo "VERIFY DEV WATCH PASS (${base})"
    exit 0
  fi
  if ! kill -0 "$watch_pid" 2>/dev/null; then
    echo "VERIFY DEV WATCH FAIL: dev-watch exited before WASM rebuild" >&2
    tail -40 "$LOG_FILE" >&2 || true
    exit 1
  fi
  sleep 2
done

echo "VERIFY DEV WATCH FAIL: WASM not rebuilt within ${TIMEOUT_SEC}s" >&2
tail -40 "$LOG_FILE" >&2 || true
exit 1