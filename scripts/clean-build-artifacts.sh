#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DRY_RUN=false
PLAYWRIGHT_DAYS=""
INCREMENTAL_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --incremental-only) INCREMENTAL_ONLY=true; shift ;;
    --playwright-days) PLAYWRIGHT_DAYS="${2:?missing days}"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

run() {
  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

if [[ -L target ]]; then
  echo "ERROR: target is a symlink; refusing cleanup" >&2
  exit 1
fi

# Fast reclaim: drop only incremental-compilation caches (the bulk of the
# accumulating bloat) and keep compiled deps so the next build stays fast.
# Use this between work sessions; reserve full `cargo clean` for tight disk.
if [[ "$INCREMENTAL_ONLY" == true ]]; then
  INC_DIRS=()
  for d in target/debug/incremental target/release/incremental; do
    [[ -d "$d" ]] && INC_DIRS+=("$d")
  done
  if [[ ${#INC_DIRS[@]} -gt 0 ]]; then
    if [[ "$DRY_RUN" == true ]]; then
      du -sh "${INC_DIRS[@]}" 2>/dev/null || true
      for d in "${INC_DIRS[@]}"; do echo "[dry-run] rm -rf $d"; done
    else
      rm -rf "${INC_DIRS[@]}"
      echo "removed incremental cache: ${INC_DIRS[*]}"
    fi
  else
    echo "no incremental caches to remove"
  fi
elif [[ -d target ]]; then
  run cargo clean
fi

# macOS ld writes multi-GB snapshots on linker failures (SymbolString.cpp / large Rust bins).
# These land in /tmp and are safe to delete; they are not reused by later builds.
LD_SNAPSHOTS=()
while IFS= read -r -d '' p; do
  LD_SNAPSHOTS+=("$p")
done < <(find /tmp -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -print0 2>/dev/null || true)
if [[ ${#LD_SNAPSHOTS[@]} -gt 0 ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    for p in "${LD_SNAPSHOTS[@]}"; do
      du -sh "$p" 2>/dev/null || echo "[dry-run] $p"
    done
  else
    for p in "${LD_SNAPSHOTS[@]}"; do
      rm -rf "$p"
    done
    echo "removed ${#LD_SNAPSHOTS[@]} linker snapshot dir(s) from /tmp"
  fi
fi

if [[ -n "$PLAYWRIGHT_DAYS" && -d .playwright-cli && ! -L .playwright-cli ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -print
  else
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -delete
  fi
fi