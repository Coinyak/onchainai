#!/usr/bin/env bash
# Local UI dev loop: Next.js HMR on PORT + Rust API on API_PORT.
#
# No cargo-leptos — the public UI is Next.js (frontend/). The Rust binary is
# API/MCP only (Axum). Next rewrites /api, /auth, /mcp to API_PROXY_TARGET.
#
# Use WHILE iterating on frontend/. Finish with ./scripts/ui-change-gate.sh
# before handoff/commit.
#
# Usage:
#   ./scripts/dev-watch.sh
#   PORT=3000 API_PORT=3001 ./scripts/dev-watch.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

if [[ "$(uname -s)" == "Darwin" && "${RUSTFLAGS:-}" != *"symbol-mangling-version"* ]]; then
  export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C symbol-mangling-version=v0"
  echo "Using macOS linker workaround: RUSTFLAGS=${RUSTFLAGS}"
fi

PORT_FROM_SHELL=false
API_PORT_FROM_SHELL=false
if [[ -n "${PORT+x}" && -n "$PORT" ]]; then
  PORT_FROM_SHELL=true
  SHELL_PORT="$PORT"
fi
if [[ -n "${API_PORT+x}" && -n "$API_PORT" ]]; then
  API_PORT_FROM_SHELL=true
  SHELL_API_PORT="$API_PORT"
fi

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
  echo "Sourced .env"
fi

if [[ "$PORT_FROM_SHELL" == "true" ]]; then
  PORT="$SHELL_PORT"
else
  PORT="${PORT:-3000}"
fi
if [[ "$API_PORT_FROM_SHELL" == "true" ]]; then
  API_PORT="$SHELL_API_PORT"
else
  API_PORT="${API_PORT:-3001}"
fi

export SKIP_CRAWLER="${SKIP_CRAWLER:-1}"
export ONCHAINAI_RELAX_RATE_LIMIT="${ONCHAINAI_RELAX_RATE_LIMIT:-1}"

if [[ -x "$ROOT/scripts/disk-guard.sh" ]]; then
  if ! "$ROOT/scripts/disk-guard.sh"; then
    echo "WARN: disk-guard reported low disk or large target/ (continuing watch loop)" >&2
  fi
fi

kill_port() {
  local p="$1"
  local pids
  pids="$(lsof -ti ":${p}" 2>/dev/null || true)"
  if [[ -n "$pids" ]]; then
    echo "Stopping stale process(es) on port ${p}: ${pids}"
    # shellcheck disable=SC2086
    kill ${pids} 2>/dev/null || true
    sleep 1
  fi
}

kill_port "$PORT"
kill_port "$API_PORT"

API_LOG="${ROOT}/target/api-dev.log"
mkdir -p target

echo "Starting Rust API on http://127.0.0.1:${API_PORT} (log: ${API_LOG})"
PORT="$API_PORT" nohup cargo run --features ssr >"$API_LOG" 2>&1 &
API_PID=$!
echo "$API_PID" >"${ROOT}/target/api-dev.pid"

cleanup() {
  echo "Stopping dev processes..."
  kill "$API_PID" 2>/dev/null || true
  kill_port "$PORT"
  kill_port "$API_PORT"
}
trap cleanup EXIT INT TERM

echo "Waiting for API on port ${API_PORT}..."
for _ in $(seq 1 120); do
  if curl -sS -o /dev/null -w "%{http_code}" "http://127.0.0.1:${API_PORT}/mcp" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' 2>/dev/null | grep -q '^200$'; then
    echo "API ready."
    break
  fi
  if ! kill -0 "$API_PID" 2>/dev/null; then
    echo "API process exited early. Log:" >&2
    tail -40 "$API_LOG" >&2
    exit 1
  fi
  sleep 1
done

echo "Starting Next.js on http://127.0.0.1:${PORT} (API proxy → ${API_PORT})"
echo "  Edit frontend/**     -> HMR reload"
echo "  Edit src/** (API)    -> restart this script (or run cargo watch separately)"
echo "  Ctrl+C to stop both. Final gate: ./scripts/ui-change-gate.sh"

cd "$ROOT/frontend"
export API_PROXY_TARGET="http://127.0.0.1:${API_PORT}"
exec npm run dev -- --port "$PORT"