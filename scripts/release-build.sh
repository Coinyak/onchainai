#!/usr/bin/env bash
# Full release build (SSR + WASM). Uses rustup cargo when available (wasm std).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

./scripts/disk-guard.sh

echo "Building release (cargo leptos build --release)..."
cargo leptos build --release

echo "Artifacts:"
ls -la target/release/onchainai target/site/pkg/onchainai.js target/site/pkg/onchainai.wasm style/output.css