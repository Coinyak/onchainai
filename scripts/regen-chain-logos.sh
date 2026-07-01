#!/usr/bin/env bash
# Regenerate all public/chains/*.svg from scripts/chain-logo-manifest.json.
#
# With a raw logo directory (official brand assets), wrap/wrap_raster entries are
# rebuilt from source. Without raw logos, those entries fall back to validating
# the committed public/chains/*.svg tiles (clone-safe).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRATCH="${ONCHAINAI_SCRATCH:-}"
RAW_DIR="${1:-}"

cd "$ROOT"

if [[ -n "$RAW_DIR" && -d "$RAW_DIR" ]]; then
  python3 scripts/wrap-chain-logos.py "$RAW_DIR"
elif [[ -n "$SCRATCH" && -d "$SCRATCH/raw-logos" ]]; then
  python3 scripts/wrap-chain-logos.py "$SCRATCH/raw-logos"
else
  echo "regen-chain-logos: no raw logo dir; validating committed public/chains tiles"
  python3 scripts/wrap-chain-logos.py --public-fallback
fi

COUNT="$(ls -1 public/chains/*.svg | wc -l | tr -d ' ')"
echo "regen-chain-logos: wrote ${COUNT} SVGs (manifest-driven, harness-round-11)"