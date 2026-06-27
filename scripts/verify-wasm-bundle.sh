#!/usr/bin/env bash
# Verify Leptos WASM hydration bundle artifacts exist after cargo leptos build.
set -euo pipefail

SITE_PKG="${1:-target/site/pkg}"
JS="${SITE_PKG}/onchainai.js"
WASM_BG="${SITE_PKG}/onchainai_bg.wasm"
WASM_ALT="${SITE_PKG}/onchainai.wasm"

if [[ ! -f "$JS" ]]; then
  echo "ERROR: missing WASM bundle artifact: $JS" >&2
  exit 1
fi

if [[ -f "$WASM_BG" ]]; then
  echo "WASM bundle OK: $JS, $WASM_BG"
  exit 0
fi

if [[ -f "$WASM_ALT" ]]; then
  # cargo-leptos 0.3 may emit onchainai.wasm while onchainai.js loads onchainai_bg.wasm
  cp "$WASM_ALT" "$WASM_BG"
  echo "WASM bundle OK: $JS, $WASM_ALT (copied to onchainai_bg.wasm for hydration)"
  exit 0
fi

echo "ERROR: missing WASM artifact (expected onchainai_bg.wasm or onchainai.wasm in $SITE_PKG)" >&2
exit 1