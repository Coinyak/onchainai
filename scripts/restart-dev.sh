#!/usr/bin/env bash
# Kill stale listeners → release build → verify → start API + Next.js → smoke test.
#
# Serves the public UI from Next.js on PORT (default 3000). Rust API listens on
# API_PORT (default 3001); Next proxies /api, /auth, /mcp via API_PROXY_TARGET.
#
# Usage:
#   ./scripts/restart-dev.sh
#   ./scripts/restart-dev.sh --no-build
#   ./scripts/restart-dev.sh --foreground
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
      sed -n '2,14p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: ${arg}" >&2
      sed -n '2,14p' "$0"
      exit 1
      ;;
  esac
done

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

PORT="${PORT:-3000}"
API_PORT="${API_PORT:-3001}"
BINARY="./target/release/onchainai"
API_LOG="${ROOT}/target/api-dev.log"
NEXT_LOG="${ROOT}/target/next-dev.log"
API_PID_FILE="${ROOT}/target/api-dev.pid"
NEXT_PID_FILE="${ROOT}/target/next-dev.pid"

kill_port() {
  local p="$1"
  local pids
  pids="$(lsof -ti ":${p}" 2>/dev/null || true)"
  if [[ -n "$pids" ]]; then
    echo "Stopping process(es) on port ${p}: ${pids}"
    # shellcheck disable=SC2086
    kill ${pids} 2>/dev/null || true
    sleep 1
    pids="$(lsof -ti ":${p}" 2>/dev/null || true)"
    if [[ -n "$pids" ]]; then
      # shellcheck disable=SC2086
      kill -9 ${pids} 2>/dev/null || true
    fi
  else
    echo "No process listening on port ${p}"
  fi
}

wait_for_bind() {
  local url="$1"
  local i
  for i in $(seq 1 90); do
    if curl -sS -o /dev/null -w "%{http_code}" "$url" 2>/dev/null | grep -q '^200$'; then
      echo "Server ready at ${url}"
      return 0
    fi
    sleep 1
  done
  echo "Timed out waiting for server at ${url}" >&2
  return 1
}

kill_port "$PORT"
kill_port "$API_PORT"

if [[ "$SKIP_BUILD" == "false" ]]; then
  echo "Running release build..."
  ./scripts/release-build.sh
else
  echo "Skipping release build (--no-build)"
fi

echo "Checking bundle coherence..."
if ! ./scripts/verify-bundle.sh; then
  echo "Bundle verify failed — API binary and Next.js build are not coherent." >&2
  echo "Run without --no-build, or: ./scripts/release-build.sh" >&2
  exit 1
fi

if [[ ! -x "$BINARY" ]]; then
  echo "Missing release binary: ${BINARY}" >&2
  echo "Run: ./scripts/release-build.sh" >&2
  exit 1
fi

if [[ ! -f frontend/.next/BUILD_ID ]]; then
  echo "Missing Next.js build: frontend/.next/BUILD_ID" >&2
  echo "Run: cd frontend && npm run build" >&2
  exit 1
fi

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
  echo "Sourced .env"
fi

export SKIP_CRAWLER="${SKIP_CRAWLER:-1}"
export ONCHAINAI_RELAX_RATE_LIMIT="${ONCHAINAI_RELAX_RATE_LIMIT:-1}"

mkdir -p target

echo "Starting API on port ${API_PORT} (log: ${API_LOG})"
PORT="$API_PORT" nohup "$BINARY" >"$API_LOG" 2>&1 &
api_pid=$!
echo "$api_pid" >"$API_PID_FILE"

api_ready() {
  curl -sS -o /dev/null -w "%{http_code}" "http://127.0.0.1:${API_PORT}/mcp" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' 2>/dev/null | grep -q '^200$'
}

echo "Waiting for API on port ${API_PORT}..."
for _ in $(seq 1 90); do
  if api_ready; then
    echo "API ready at http://127.0.0.1:${API_PORT}"
    break
  fi
  sleep 1
done
if ! api_ready; then
  echo "---- API log (last 40 lines) ----" >&2
  tail -40 "$API_LOG" >&2
  exit 1
fi

echo "Starting Next.js on port ${PORT} (log: ${NEXT_LOG})"
(
  cd frontend
  export API_PROXY_TARGET="http://127.0.0.1:${API_PORT}"
  export PORT
  nohup npm run start -- --port "$PORT" >"$NEXT_LOG" 2>&1 &
  echo $! >"$NEXT_PID_FILE"
)

next_pid="$(cat "$NEXT_PID_FILE")"
echo "API PID ${api_pid}, Next PID ${next_pid}"

if [[ "$FOREGROUND" == "true" ]]; then
  trap 'kill "$api_pid" "$next_pid" 2>/dev/null || true' EXIT
fi

if ! wait_for_bind "http://127.0.0.1:${PORT}/"; then
  echo "---- Next log (last 40 lines) ----" >&2
  tail -40 "$NEXT_LOG" >&2
  exit 1
fi

export ONCHAINAI_SMOKE_API_BASE="http://127.0.0.1:${API_PORT}"
./scripts/smoke-test.sh "http://127.0.0.1:${PORT}"

if [[ "$FOREGROUND" == "true" ]]; then
  echo "Foreground servers running (API ${api_pid}, Next ${next_pid}); press Ctrl+C to stop."
  wait "$next_pid"
else
  echo "Dev stack running. UI: http://127.0.0.1:${PORT}  API: http://127.0.0.1:${API_PORT}"
  echo "Logs: ${NEXT_LOG} ${API_LOG}"
  echo "Stop: kill \$(cat ${NEXT_PID_FILE}) \$(cat ${API_PID_FILE})"
fi