#!/usr/bin/env bash
# Final local gate for UI/auth/routing changes.
#
# Runs one coherent Leptos release build, restarts the matching binary, then
# verifies curl smoke, browser interactivity, auth shell behavior, and
# desktop/mobile screenshots. Use this instead of finishing UI work with a
# standalone cargo build or cargo run.
#
# Usage:
#   ./scripts/ui-change-gate.sh
#   ./scripts/ui-change-gate.sh --no-build
#   ./scripts/ui-change-gate.sh --skip-auth
#   ./scripts/ui-change-gate.sh --skip-snapshots
#   ./scripts/ui-change-gate.sh --tier smoke
#   ./scripts/ui-change-gate.sh --check-only
#   ./scripts/ui-change-gate.sh --harness-only   # alias for --check-only
#   ./scripts/ui-change-gate.sh --port 3001
#   ./scripts/ui-change-gate.sh --base http://127.0.0.1:3000
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -n "${PORT+x}" ]]; then
  PORT_FROM_ENV=true
  PORT="${PORT}"
else
  PORT_FROM_ENV=false
  PORT="3000"
fi
BASE=""
BASE_PROVIDED=false
PORT_FROM_ARG=false
SKIP_BUILD=false
SKIP_AUTH=false
SKIP_SNAPSHOTS=false
CHECK_ONLY=false
TIER="full"
SNAPSHOT_DIR=".playwright-cli/ui-snapshots"

usage() {
  sed -n '2,16p' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE="${2:-}"
      if [[ -z "$BASE" ]]; then
        echo "Missing value for --base" >&2
        exit 2
      fi
      shift 2
      ;;
    --port)
      PORT="${2:-}"
      if [[ ! "$PORT" =~ ^[0-9]+$ ]]; then
        echo "Missing or invalid value for --port" >&2
        exit 2
      fi
      PORT_FROM_ARG=true
      shift 2
      ;;
    --no-build)
      SKIP_BUILD=true
      shift
      ;;
    --skip-auth)
      SKIP_AUTH=true
      shift
      ;;
    --skip-snapshots)
      SKIP_SNAPSHOTS=true
      shift
      ;;
    --check-only|--harness-only)
      CHECK_ONLY=true
      shift
      ;;
    --tier)
      TIER="${2:-}"
      if [[ "$TIER" != "full" && "$TIER" != "smoke" ]]; then
        echo "Missing or invalid value for --tier (full|smoke)" >&2
        exit 2
      fi
      shift 2
      ;;
    --out)
      SNAPSHOT_DIR="${2:-}"
      if [[ -z "$SNAPSHOT_DIR" ]]; then
        echo "Missing value for --out" >&2
        exit 2
      fi
      shift 2
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

if [[ -z "$BASE" ]]; then
  BASE="http://127.0.0.1:${PORT}"
else
  BASE_PROVIDED=true
  BASE="${BASE%/}"
fi

if [[ "$BASE_PROVIDED" == "true" ]]; then
  if [[ ! "$BASE" =~ ^http://(localhost|127\.0\.0\.1|\[::1\]):([0-9]+)$ ]]; then
    echo "Invalid --base for this local restart gate: ${BASE}" >&2
    echo "Use http://127.0.0.1:PORT, or pass --port PORT and let the script derive the base URL." >&2
    exit 2
  fi
  base_port="${BASH_REMATCH[2]}"
  if [[ "$PORT" != "$base_port" ]]; then
    if [[ "$PORT_FROM_ARG" == "false" && "$PORT_FROM_ENV" == "false" ]]; then
      PORT="$base_port"
    else
      echo "PORT (${PORT}) does not match --base port (${base_port})." >&2
      echo "Use --port ${base_port}, set PORT=${base_port}, or omit --base." >&2
      exit 2
    fi
  fi
fi

export PORT

if [[ "$CHECK_ONLY" == "true" ]]; then
  echo "AGENT HARNESS ONLY PASS (no build/browser; PORT=${PORT}, BASE=${BASE})"
  exit 0
fi

if [[ "$TIER" == "smoke" ]]; then
  SKIP_AUTH=true
  SKIP_SNAPSHOTS=true
fi

total_steps=6
if [[ "$TIER" == "smoke" ]]; then
  total_steps=3
fi

echo "UI change gate starting for ${BASE} (tier=${TIER})"
echo "Step 1/${total_steps}: agent harness contract"
./scripts/agent-harness-check.sh

echo "Step 2/${total_steps}: coherent release build, restart, and curl smoke"
if [[ "$SKIP_BUILD" == "true" ]]; then
  ./scripts/restart-dev.sh --no-build
else
  ./scripts/restart-dev.sh
fi

echo "Step 3/${total_steps}: bundle coherence"
./scripts/verify-bundle.sh

if [[ "$TIER" == "smoke" ]]; then
  echo ""
  echo "UI CHANGE GATE PASS (tier=smoke: build + curl smoke only)"
  echo "For browser/auth/visual QA run: ./scripts/ui-change-gate.sh --tier full"
  exit 0
fi

echo "Step 4/${total_steps}: browser smoke + click test (single Playwright session)"
ONCHAINAI_SCRATCH="${SNAPSHOT_DIR%/ui-snapshots}" node scripts/ui-browser-gate.mjs "$BASE"

if [[ "$SKIP_AUTH" == "true" ]]; then
  echo "Step 5/${total_steps}: local auth smoke skipped"
elif [[ -f scripts/local-auth-smoke.mjs ]]; then
  echo "Step 5/${total_steps}: local auth smoke"
  node scripts/local-auth-smoke.mjs "$BASE"
else
  echo "Step 5/${total_steps}: local auth smoke not present; skipped"
fi

if [[ "$SKIP_SNAPSHOTS" == "true" ]]; then
  echo "Step 6/${total_steps}: visual snapshots skipped"
else
  echo "Step 6/${total_steps}: desktop/mobile visual snapshots"
  node scripts/visual-snapshots.mjs "$BASE" --out "$SNAPSHOT_DIR"
fi

echo ""
echo "UI CHANGE GATE PASS (tier=full)"
echo "Snapshot directory: ${SNAPSHOT_DIR}"
echo "If an existing browser tab still looks stale, hard refresh it (Cmd+Shift+R)."
