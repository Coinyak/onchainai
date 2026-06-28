#!/usr/bin/env bash
# Universal UI bundle staleness check (any coding tool, human or agent).
#
# Detects the classic OnchainAI failure (docs/BUILD_DEPLOY_RULES.md §3): UI /
# Leptos source was edited but the coherent build never ran, so the browser
# still hydrates the OLD bundle (stale layout, dead sidebar, buttons not
# clickable).
#
# Enforcement surfaces (tool-agnostic first):
#   - Git pre-commit (scripts/git-hooks/pre-commit) — blocks stale UI commits
#   - Manual / CI: ./scripts/ui-staleness-check.sh [--staged|--worktree]
#
# Optional IDE stop hooks (when the tool supports them):
#   - .cursor/hooks.json (stop + subagentStop)
#   - .claude/settings.json (Stop)
#
# Scope: hydrate/WASM sources from src/lib.rs (excludes #[cfg(ssr)]-only trees
# like src/crawler/, src/auth/routes.rs, src/main.rs).
# style/output.css is NOT checked — served live (ServeFile in src/lib.rs).
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
      sed -n '2,30p' "$0"
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

WASM="target/site/pkg/onchainai.wasm"

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
  bundle:  ${WASM}
  mode:    ${MODE}

The running site may hydrate a STALE bundle (old layout, dead sidebar, buttons
not clickable). Before finishing UI / auth / routing work, make the bundle
coherent:

  ./scripts/dev-watch.sh       # iterating: auto rebuild (SSR+WASM) + live reload
  ./scripts/ui-change-gate.sh  # final gate: release build + smoke + screenshots

If this is NOT a UI change that needs browser verification, or the build is
blocked (disk/linker), say so explicitly instead of implying UI QA passed, and
re-run with ONCHAINAI_SKIP_STALENESS=1 to proceed.
EOF
  exit 2
}

if [[ ! -f "$WASM" ]]; then
  if [[ "$MODE" == "staged" ]]; then
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      stale_path="$path"
      stale_reason="Staged UI source has no built WASM bundle yet"
      break
    done < <(collect_staged_ui_paths)
  else
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      stale_path="$path"
      stale_reason="UI source changed but no WASM bundle exists yet"
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
    if [[ -f "$path" && "$path" -nt "$WASM" ]]; then
      stale_path="$path"
      stale_reason="Staged UI source is newer than the built WASM bundle"
      break
    fi
  done < <(collect_staged_ui_paths)
else
  for base in "${ui_watch_paths[@]}"; do
    if [[ -f "$base" ]]; then
      if [[ "$base" -nt "$WASM" ]]; then
        stale_path="$base"
        break
      fi
    elif [[ -d "$base" ]]; then
      stale_path="$(find "$base" -name '*.rs' -newer "$WASM" -print -quit 2>/dev/null || true)"
      if [[ -n "$stale_path" ]]; then
        break
      fi
    fi
  done
  if [[ -n "$stale_path" ]]; then
    stale_reason="UI source is newer than the built WASM bundle"
  fi
fi

if [[ -z "$stale_path" ]]; then
  exit 0
fi

report_stale "$stale_reason" "$stale_path"