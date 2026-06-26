#!/usr/bin/env bash
# Verify Leptos WASM hydration bundle artifacts exist after cargo leptos build.
set -euo pipefail

SITE_PKG="${1:-target/site/pkg}"
JS="${SITE_PKG}/onchainai.js"
WASM="${SITE_PKG}/onchainai_bg.wasm"

missing=0
for artifact in "$JS" "$WASM"; do
  if [[ ! -f "$artifact" ]]; then
    echo "ERROR: missing WASM bundle artifact: $artifact" >&2
    missing=1
  fi
done

if [[ "$missing" -ne 0 ]]; then
  echo "WASM bundle verification failed. Full cargo leptos build is required." >&2
  exit 1
fi

echo "WASM bundle OK: $JS, $WASM"