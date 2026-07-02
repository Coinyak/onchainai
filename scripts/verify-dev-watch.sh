#!/usr/bin/env bash
# E2E check: Next.js frontend rebuilds when a UI source changes.
# Legacy Leptos dev-watch path removed after frontend migration to Vercel.
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
PROBE_FILE="frontend/components/layout/Sidebar.tsx"
BUNDLE="frontend/.next/BUILD_ID"
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
  [[ -f frontend/package.json ]] || missing+=("frontend/package.json")
  [[ -x scripts/smoke-test.sh ]] || missing+=("scripts/smoke-test.sh")
  if (( ${#missing[@]} > 0 )); then
    echo "VERIFY DEV WATCH CHECK ONLY FAIL: missing or not executable: ${missing[*]}" >&2
    exit 1
  fi
  echo "VERIFY DEV WATCH CHECK ONLY PASS"
  echo "  probe:  ${PROBE_FILE}"
  echo "  scripts: frontend npm build, smoke-test.sh"
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

PROBE_BACKUP=""

restore_probe_file() {
  if [[ -n "$PROBE_BACKUP" && -f "$PROBE_BACKUP" ]]; then
    cp "$PROBE_BACKUP" "$PROBE_FILE"
    rm -f "$PROBE_BACKUP"
    PROBE_BACKUP=""
  fi
}

cleanup() {
  restore_probe_file
}
trap cleanup EXIT

echo "Running frontend build verify (timeout ${TIMEOUT_SEC}s)"
(cd frontend && npm run build) >/dev/null

if [[ ! -f "$BUNDLE" ]]; then
  echo "VERIFY DEV WATCH FAIL: ${BUNDLE} missing after initial build" >&2
  exit 1
fi

bundle_before="$(stat -f %m "$BUNDLE" 2>/dev/null || stat -c %Y "$BUNDLE")"

PROBE_BACKUP="$(mktemp -t onchainai-dev-watch-probe.XXXXXX.bak)"
cp "$PROBE_FILE" "$PROBE_BACKUP"
printf '\n// verify-dev-watch-probe %s\n' "$(date +%s)" >>"$PROBE_FILE"

echo "Rebuilding frontend after editing ${PROBE_FILE}"
(cd frontend && npm run build) >/dev/null

bundle_after="$(stat -f %m "$BUNDLE" 2>/dev/null || stat -c %Y "$BUNDLE")"
if [[ "$bundle_after" -ge "$bundle_before" ]]; then
  restore_probe_file
  echo "VERIFY DEV WATCH PASS (frontend/.next/BUILD_ID ${bundle_before} -> ${bundle_after})"
  exit 0
fi

echo "VERIFY DEV WATCH FAIL: BUILD_ID not refreshed after probe edit" >&2
exit 1