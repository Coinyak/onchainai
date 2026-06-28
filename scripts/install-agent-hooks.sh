#!/usr/bin/env bash
# Install universal UI enforcement for ANY coding tool (Codex, Copilot, Droid,
# Cursor, Claude Code, Grok, VS Code, terminal, …).
#
# Primary (tool-agnostic): Git pre-commit via core.hooksPath → blocks stale UI
# commits no matter which agent edited the files.
#
# Optional (IDE-only, when supported): committed stop hooks for earlier feedback
# before commit — .cursor/hooks.json, .claude/settings.json.
#
# Usage:
#   ./scripts/install-agent-hooks.sh
#   ./scripts/install-agent-hooks.sh --check-only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CHECK_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --check-only) CHECK_ONLY=1 ;;
    -h|--help)
      sed -n '2,13p' "$0"
      exit 0
      ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

fail() {
  echo "INSTALL AGENT HOOKS FAIL: $*" >&2
  exit 1
}

pass() {
  echo "INSTALL AGENT HOOKS PASS"
}

require_file() {
  [[ -e "$1" ]] || fail "missing required file: $1"
}

require_exec() {
  [[ -x "$1" ]] || fail "not executable: $1 (run without --check-only to chmod)"
}

require_file "scripts/git-hooks/pre-commit"
require_file "scripts/git-hooks/pre-push"
require_file "scripts/ui-staleness-check.sh"
require_file "scripts/dev-watch.sh"
require_file ".cursor/hooks.json"
require_file ".claude/settings.json"
require_file ".cursor/hooks/ui-staleness-stop.sh"

if [[ "$CHECK_ONLY" == "1" ]]; then
  require_exec "scripts/git-hooks/pre-commit"
  require_exec "scripts/git-hooks/pre-push"
  require_exec "scripts/ui-staleness-check.sh"
  require_exec "scripts/dev-watch.sh"
  require_exec ".cursor/hooks/ui-staleness-stop.sh"
  hooks_path="$(git config --local --get core.hooksPath 2>/dev/null || true)"
  [[ "$hooks_path" == "scripts/git-hooks" ]] || \
    fail "git core.hooksPath must be 'scripts/git-hooks' (run install without --check-only)"
  pass
  exit 0
fi

chmod +x \
  scripts/git-hooks/pre-commit \
  scripts/git-hooks/pre-push \
  scripts/ui-staleness-check.sh \
  scripts/dev-watch.sh \
  .cursor/hooks/ui-staleness-stop.sh

git config --local core.hooksPath scripts/git-hooks

echo "Universal enforcement installed (works for any coding tool)."
echo ""
echo "  Git pre-commit:  scripts/git-hooks/pre-commit (staged UI)"
echo "  Git pre-push:    scripts/git-hooks/pre-push (worktree UI)"
echo "  Checker:         scripts/ui-staleness-check.sh"
echo ""
echo "Optional IDE stop hooks (earlier feedback, if your tool supports them):"
echo "  .cursor/hooks.json       — Cursor / Grok (Third-party skills for Claude hooks)"
echo "  .claude/settings.json    — Claude Code Stop"
echo "  Personal overrides:     .claude/settings.local.json (gitignored)"
echo ""
echo "Fast UI loop:  ./scripts/dev-watch.sh"
echo "Final gate:    ./scripts/ui-change-gate.sh"

pass