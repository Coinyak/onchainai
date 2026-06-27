#!/usr/bin/env bash
# Regenerate all public/chains/*.svg from scripts/chain-logo-manifest.json.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRATCH="${ONCHAINAI_SCRATCH:-/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/T/grok-goal-11e98898edeb/implementer}"
RAW_DIR="${1:-$SCRATCH/raw-logos}"

cd "$ROOT"
python3 scripts/wrap-chain-logos.py "$RAW_DIR"
COUNT="$(ls -1 public/chains/*.svg | wc -l | tr -d ' ')"
echo "regen-chain-logos: wrote ${COUNT} SVGs (manifest-driven, harness-round-8)"