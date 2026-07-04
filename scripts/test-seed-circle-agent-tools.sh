#!/usr/bin/env bash
# Self-contained test for scripts/seed-circle-agent-tools.mjs (PR-1 manifest).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

node scripts/test-seed-circle-agent-tools.mjs
echo "SEED CIRCLE TEST PASS"