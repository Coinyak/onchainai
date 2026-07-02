#!/usr/bin/env bash
# Universal UI bundle staleness check (any coding tool, human or agent).
#
# Detects stale Next.js frontend: UI source was edited but no coherent
# `npm run build` ran, so preview/production may serve an old bundle.
#
# Enforcement surfaces (tool-agnostic first):
#   - Git pre-commit (scripts/git-hooks/pre-commit) — blocks stale UI commits
#   - Manual / CI: ./scripts/ui-staleness-check.sh [--staged|--worktree]
#
# Optional IDE stop hooks (when the tool supports them):
#   - .cursor/hooks.json (stop + subagentStop)
#   - .claude/settings.json (Stop)
#
# Scope: Next.js UI under frontend/ (app, components, lib, config).
#
# Modes:
#   (default) --worktree  any UI source newer than bundle (or changed when bundle missing)
#   --staged              only staged UI paths in the current commit
#
#   Exit 0 = OK. Exit 2 = stale (git hook / IDE stop hooks block).
#
# Bypass: ONCHAINAI_SKIP_STALENESS=1
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MODE="worktree"
for arg in "$@"; do
  case "$arg" in
    --staged) MODE="staged" ;;
    --worktree) MODE="worktree" ;;
    -h|--help)
      sed -n '2,28p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

if [[ "${ONCHAINAI_SKIP_STALENESS:-0}" == "1" ]]; then
  echo "[ui-staleness] ONCHAINAI_SKIP_STALENESS=1 — bypassing staleness check (mode: ${MODE})" >&2
  exit 0
fi

BUNDLE="frontend/.next/BUILD_ID"

INC="$(dirname "$0")/ui-watch-paths.inc.sh"
if [[ ! -f "$INC" ]]; then
  echo "[ui-staleness] missing ${INC}; run: node scripts/sync-ui-watch-paths.mjs" >&2
  exit 1
fi
# shellcheck source=scripts/ui-watch-paths.inc.sh
source "$INC"

stale_reason=""
stale_path=""

collect_staged_ui_paths() {
  git diff --cached --name-only --diff-filter=ACMR | grep -E "$ui_path_re" || true
}

collect_worktree_ui_paths() {
  {
    git diff --name-only --diff-filter=ACMR
    git diff --cached --name-only --diff-filter=ACMR
    git ls-files --others --exclude-standard
  } | grep -E "$ui_path_re" | sort -u || true
}

report_stale() {
  local reason="$1"
  local path="$2"
  cat >&2 <<EOF
[ui-staleness] ${reason}
  changed: ${path}
  bundle:  ${BUNDLE}
  mode:    ${MODE}

The site may serve a STALE Next.js bundle. Before finishing UI / auth / routing work,
make the frontend coherent:

  cd frontend && npm run dev     # iterating: HMR on :3000
  cd frontend && npm run build   # compile check before commit
  ./scripts/ui-change-gate.sh  # final gate: smoke + browser checks

If this is NOT a UI change that needs browser verification, or the build is
blocked, say so explicitly instead of implying UI QA passed, and re-run with
ONCHAINAI_SKIP_STALENESS=1 to proceed.
EOF
  exit 2
}

find_newer_ui_file() {
  local base="$1"
  if [[ -f "$base" ]]; then
    if [[ "$base" -nt "$BUNDLE" ]]; then
      echo "$base"
    fi
    return
  fi
  if [[ -d "$base" ]]; then
    find "$base" \( -name '*.ts' -o -name '*.tsx' -o -name '*.css' -o -name '*.mjs' \) \
      -newer "$BUNDLE" -print -quit 2>/dev/null || true
  fi
}

if [[ ! -f "$BUNDLE" ]]; then
  if [[ "$MODE" == "staged" ]]; then
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      stale_path="$path"
      stale_reason="Staged UI source has no Next.js build yet (missing BUILD_ID)"
      break
    done < <(collect_staged_ui_paths)
  else
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      stale_path="$path"
      stale_reason="UI source changed but no Next.js build exists yet"
      break
    done < <(collect_worktree_ui_paths)
  fi

  if [[ -n "$stale_path" ]]; then
    report_stale "$stale_reason" "$stale_path"
  fi
  exit 0
fi

if [[ "$MODE" == "staged" ]]; then
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    if [[ -f "$path" && "$path" -nt "$BUNDLE" ]]; then
      stale_path="$path"
      stale_reason="Staged UI source is newer than the last Next.js build"
      break
    fi
  done < <(collect_staged_ui_paths)
else
  for base in "${ui_watch_paths[@]}"; do
    stale_path="$(find_newer_ui_file "$base")"
    if [[ -n "$stale_path" ]]; then
      stale_reason="UI source is newer than the last Next.js build"
      break
    fi
  done
fi

if [[ -z "$stale_path" ]]; then
  exit 0
fi

report_stale "$stale_reason" "$stale_path"