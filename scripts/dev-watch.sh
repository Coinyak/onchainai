#!/usr/bin/env bash
# Fast, COHERENT local dev loop for UI work (Leptos HMR-equivalent).
#
# Runs `cargo leptos watch`, which rebuilds the SSR binary AND the WASM/JS
# bundle together on every save and live-reloads the browser. This keeps SSR
# markup and hydration WASM from diverging — the #1 cause of "I changed the UI
# but the site looks stale / sidebar dead / buttons not clickable" (see
# docs/BUILD_DEPLOY_RULES.md §3).
#
# Use this WHILE iterating. Finish with ./scripts/ui-change-gate.sh as the
# final gate before handoff/commit (release build + full smoke + screenshots).
#
# Why not `cargo build --features ssr`? That rebuilds the server ONLY; the WASM
# bundle and SSR markup drift apart and the browser hydrates a stale UI. Never
# use it as a way to "preview" UI changes — use this watch loop instead.
#
# CSS note: style/output.css is hand-authored and served live by Axum
# (ServeFile in src/lib.rs). Editing it only needs a browser refresh.
#
# Usage:
#   ./scripts/dev-watch.sh
#   PORT=3000 ./scripts/dev-watch.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

# macOS linker workaround (same as release-build.sh): Apple clang can fail with
# makeSymbolStringInPlace unless symbol mangling v0 is forced.
if [[ "$(uname -s)" == "Darwin" && "${RUSTFLAGS:-}" != *"symbol-mangling-version"* ]]; then
  export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C symbol-mangling-version=v0"
  echo "Using macOS linker workaround: RUSTFLAGS=${RUSTFLAGS}"
fi

PORT_FROM_SHELL=false
if [[ -n "${PORT+x}" && -n "$PORT" ]]; then
  PORT_FROM_SHELL=true
  SHELL_PORT="$PORT"
fi

# Load .env before defaulting PORT so .env values apply unless the shell overrides.
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

export SKIP_CRAWLER="${SKIP_CRAWLER:-1}"
export ONCHAINAI_PKG_NO_CACHE="${ONCHAINAI_PKG_NO_CACHE:-1}"
# cargo-leptos reads LEPTOS_SITE_ADDR to override site-addr from Cargo.toml.
export LEPTOS_SITE_ADDR="127.0.0.1:${PORT}"

if [[ -x "$ROOT/scripts/disk-guard.sh" ]]; then
  if ! "$ROOT/scripts/disk-guard.sh"; then
    echo "WARN: disk-guard reported low disk or large target/ (continuing watch loop)" >&2
  fi
fi

# Kill any stale server on the watch port — stale processes are the #1 cause of
# "changes not showing" (docs/BUILD_DEPLOY_RULES.md §2).
pids="$(lsof -ti ":${PORT}" 2>/dev/null || true)"
if [[ -n "$pids" ]]; then
  echo "Stopping stale process(es) on port ${PORT}: ${pids}"
  # shellcheck disable=SC2086
  kill ${pids} 2>/dev/null || true
  sleep 1
fi

echo "Starting cargo leptos watch on http://127.0.0.1:${PORT} (reload-port 3001)."
echo "  Edit src/**.rs        -> auto rebuild (SSR + WASM) + browser live-reload"
echo "  Edit style/output.css -> just refresh the browser (served live)"
echo "  Ctrl+C to stop. Final gate before commit: ./scripts/ui-change-gate.sh"
exec cargo leptos watch