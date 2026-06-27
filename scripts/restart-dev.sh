#!/usr/bin/env bash
# Kill :3000 → release build → verify bundle → restart release binary → smoke test.
#
# Usage:
#   ./scripts/restart-dev.sh              # full workflow (build + restart + smoke)
#   ./scripts/restart-dev.sh --no-build   # skip build; verify existing artifacts only
#   ./scripts/restart-dev.sh --foreground # build + restart in foreground (blocks; Ctrl+C stops)
#
# Prerequisites:
#   .env                                    # optional; sourced for DATABASE_URL and other secrets
#
# Environment:
#   PORT=3000                               # listen port (default 3000)
#   SKIP_CRAWLER=1                          # set by default for faster local dev
#   LEPTOS_SITE_ROOT=target/site            # served static/hydration assets
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FOREGROUND=false
SKIP_BUILD=false
for arg in "$@"; do
  case "$arg" in
    --foreground) FOREGROUND=true ;;
    --no-build) SKIP_BUILD=true ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: ${arg}" >&2
      sed -n '2,12p' "$0"
      exit 1
      ;;
  esac
done

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

PORT="${PORT:-3000}"
BINARY="./target/release/onchainai"
LOG_FILE="${ROOT}/target/dev-server.log"
PID_FILE="${ROOT}/target/dev-server.pid"

kill_port() {
  local pids
  pids="$(lsof -ti ":${PORT}" 2>/dev/null || true)"
  if [[ -n "$pids" ]]; then
    echo "Stopping process(es) on port ${PORT}: ${pids}"
    # shellcheck disable=SC2086
    kill ${pids} 2>/dev/null || true
    sleep 1
    pids="$(lsof -ti ":${PORT}" 2>/dev/null || true)"
    if [[ -n "$pids" ]]; then
      # shellcheck disable=SC2086
      kill -9 ${pids} 2>/dev/null || true
    fi
  else
    echo "No process listening on port ${PORT}"
  fi
}

wait_for_bind() {
  local url="http://127.0.0.1:${PORT}/"
  local i
  for i in $(seq 1 60); do
    if curl -sS -o /dev/null -w "%{http_code}" "$url" 2>/dev/null | grep -q '^200$'; then
      echo "Server ready at ${url}"
      return 0
    fi
    sleep 1
  done
  echo "Timed out waiting for server on port ${PORT}" >&2
  if [[ -f "$LOG_FILE" ]]; then
    echo "---- ${LOG_FILE} (last 40 lines) ----" >&2
    tail -40 "$LOG_FILE" >&2
  fi
  return 1
}

# Kill stale :3000 first — #1 cause of "changes not showing" (see BUILD_DEPLOY_RULES.md §2).
kill_port

if [[ "$SKIP_BUILD" == "false" ]]; then
  echo "Running release build..."
  ./scripts/release-build.sh
else
  echo "Skipping release build (--no-build)"
fi

echo "Checking bundle coherence..."
if ! ./scripts/verify-bundle.sh; then
  echo "Bundle verify failed — binary and WASM/JS are not from one build." >&2
  echo "Run without --no-build, or: ./scripts/release-build.sh" >&2
  exit 1
fi

if [[ ! -x "$BINARY" ]]; then
  echo "Missing release binary: ${BINARY}" >&2
  echo "Run: ./scripts/release-build.sh" >&2
  exit 1
fi

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
  echo "Sourced .env"
fi

export PORT
export LEPTOS_SITE_ROOT="${LEPTOS_SITE_ROOT:-target/site}"
export SKIP_CRAWLER="${SKIP_CRAWLER:-1}"

mkdir -p target

if [[ "$FOREGROUND" == "true" ]]; then
  echo "Starting ${BINARY} in foreground (LEPTOS_SITE_ROOT=${LEPTOS_SITE_ROOT}, SKIP_CRAWLER=${SKIP_CRAWLER})"
  echo "Note: terminal blocks until Ctrl+C; smoke test runs after bind in a subshell."
  "${BINARY}" &
  server_pid=$!
  echo "$server_pid" >"$PID_FILE"
  trap 'kill "$server_pid" 2>/dev/null || true' EXIT
  wait_for_bind
  ./scripts/smoke-test.sh "http://127.0.0.1:${PORT}"
  echo "Foreground server PID ${server_pid}; press Ctrl+C to stop."
  wait "$server_pid"
else
  echo "Starting ${BINARY} in background (log: ${LOG_FILE})"
  nohup "${BINARY}" >"$LOG_FILE" 2>&1 &
  server_pid=$!
  echo "$server_pid" >"$PID_FILE"
  echo "Server PID ${server_pid}"
  wait_for_bind
  ./scripts/smoke-test.sh "http://127.0.0.1:${PORT}"
  echo "Dev server running (PID ${server_pid}). Logs: ${LOG_FILE}"
  echo "Stop: kill \$(cat ${PID_FILE})  # or ./scripts/restart-dev.sh"
fi