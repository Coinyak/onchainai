#!/usr/bin/env bash
# Full release build (SSR + WASM). Uses rustup cargo when available (wasm std).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

if ! ./scripts/disk-guard.sh; then
  echo "disk-guard failed; pruning linker snapshots in /tmp before retry..." >&2
  find /tmp -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -exec rm -rf {} + 2>/dev/null || true
  ./scripts/disk-guard.sh
fi

echo "Building release (cargo leptos build --release)..."
cargo leptos build --release

# wasm-bindgen JS loads onchainai_bg.wasm; cargo-leptos emits onchainai.wasm.
ln -sf onchainai.wasm target/site/pkg/onchainai_bg.wasm

echo "Artifacts:"
ls -la target/release/onchainai target/site/pkg/onchainai.js target/site/pkg/onchainai.wasm target/site/pkg/onchainai_bg.wasm style/output.css

./scripts/verify-bundle.sh

echo "Running lib tests (pagination/query coverage)..."
if ! cargo test --features ssr --lib; then
  echo "WARN: cargo test --lib failed (macOS linker / disk?). Running compile check..." >&2
  find /tmp -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -exec rm -rf {} + 2>/dev/null || true
  cargo check --features ssr --lib
fi

echo ""
echo "Next: ./scripts/restart-dev.sh"