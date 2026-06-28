#!/usr/bin/env bash
# Optional session bootstrap (any coding tool). Not required every task.
#
# Usage: ./scripts/agent-start.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== OnchainAI agent start ==="
git status --short || true

if ! ./scripts/install-agent-hooks.sh --check-only >/dev/null 2>&1; then
  ./scripts/install-agent-hooks.sh
else
  echo "Agent hooks: installed"
fi

if [[ -x scripts/disk-guard.sh ]]; then
  ./scripts/disk-guard.sh || echo "disk-guard: warning (continuing)" >&2
fi

echo ""
echo "UI work:  ./scripts/dev-watch.sh"
echo "Finish:   ./scripts/ui-change-gate.sh"
echo "Fast gate: ./scripts/ui-change-gate.sh --tier smoke"
echo "Harness:  ./scripts/agent-harness-check.sh"