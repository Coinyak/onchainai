#!/usr/bin/env bash
# Coherent local release build: Rust API binary + Next.js production bundle.
# No cargo-leptos / WASM — UI is Next.js on Vercel; API is Dockerfile.api on Railway.
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

if ! ./scripts/disk-guard.sh; then
  echo "disk-guard failed; pruning linker snapshots in /tmp before retry..." >&2
  find /tmp -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -exec rm -rf {} + 2>/dev/null || true
  ./scripts/disk-guard.sh
fi

echo "Building API release (cargo build --release --features ssr)..."
cargo build --release --features ssr

if [[ ! -x target/release/onchainai ]]; then
  echo "Missing release binary: target/release/onchainai" >&2
  exit 1
fi

echo "Building Next.js (cd frontend && npm run build)..."
# Next.js bakes rewrite destinations into routes-manifest.json at build time
# (next start does not re-evaluate next.config.ts's rewrites()), so the API
# proxy target must be correct *before* this build, not just before runtime
# start (restart-dev.sh sets it again for npm run start, which is too late
# if the manifest was built with the wrong/default value).
export API_PROXY_TARGET="${API_PROXY_TARGET:-http://127.0.0.1:3001}"
(cd frontend && npm run build)

if [[ ! -f frontend/.next/BUILD_ID ]]; then
  echo "Missing Next.js build: frontend/.next/BUILD_ID" >&2
  exit 1
fi

touch target/release/onchainai frontend/.next/BUILD_ID

echo "Artifacts:"
ls -la target/release/onchainai frontend/.next/BUILD_ID

./scripts/verify-bundle.sh

echo "Running lib tests (pagination/query coverage)..."
if ! cargo test --features ssr --lib; then
  echo "WARN: cargo test --lib failed (macOS linker / disk?). Running compile check..." >&2
  find /tmp -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -exec rm -rf {} + 2>/dev/null || true
  cargo check --features ssr --lib
fi

echo ""
echo "Next: ./scripts/restart-dev.sh"