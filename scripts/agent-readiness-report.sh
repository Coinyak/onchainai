#!/usr/bin/env bash
# Wrapper for the Droid-style agent readiness report.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

exec node scripts/agent-readiness-report.mjs "$@"
