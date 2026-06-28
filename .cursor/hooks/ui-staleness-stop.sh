#!/usr/bin/env bash
# Cursor stop/subagentStop hook: block finishing on a stale Leptos WASM bundle.
#
# Cursor sends JSON on stdin; ui-staleness-check.sh only needs exit codes.
# Exit 0 = OK to stop. Exit 2 = block (Cursor + Claude Code both honor this).
set -uo pipefail

cat > /dev/null

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec "$ROOT/scripts/ui-staleness-check.sh"