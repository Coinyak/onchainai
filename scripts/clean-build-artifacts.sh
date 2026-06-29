#!/usr/bin/env bash
# Safe local cleanup. Usage:
#   --incremental-only   drop target/*/incremental/ only (fast; keeps deps)
#   --snapshots-only     sweep linker snapshots only; never touch target/
#   --dry-run            print actions without deleting
#   --playwright-days N  prune .playwright-cli files older than N days
# Default (no flags): cargo clean + linker snapshots.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DRY_RUN=false
PLAYWRIGHT_DAYS=""
INCREMENTAL_ONLY=false
SNAPSHOTS_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --incremental-only) INCREMENTAL_ONLY=true; shift ;;
    --snapshots-only) SNAPSHOTS_ONLY=true; shift ;;
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
if [[ "$SNAPSHOTS_ONLY" == true ]]; then
  : # skip all target/ cleanup; only the linker-snapshot sweep below runs
elif [[ "$INCREMENTAL_ONLY" == true ]]; then
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
# These land in the temp dir and are safe to delete; they are not reused by later builds.
#
# IMPORTANT: on macOS /tmp is a symlink to /private/tmp, and BSD `find` does NOT
# descend a symlinked start path without -H/-L. `find /tmp ...` therefore matched
# NOTHING and this sweep was a silent no-op for every macOS build (snapshots piled
# to tens of GB). Resolve each temp dir to its real path before scanning, and dedupe.
TMP_DIRS=()
for t in /tmp /private/tmp "${TMPDIR:-}"; do
  [[ -n "$t" && -d "$t" ]] || continue
  rp="$(cd "$t" 2>/dev/null && pwd -P)" || continue
  dup=false
  # Quoted array iteration, guarded so an empty TMP_DIRS does not trip `set -u`
  # on bash 3.2 (macOS default). `"${TMP_DIRS[@]}"` is only expanded when non-empty.
  if [[ ${#TMP_DIRS[@]} -gt 0 ]]; then
    for e in "${TMP_DIRS[@]}"; do [[ "$e" == "$rp" ]] && { dup=true; break; }; done
  fi
  [[ "$dup" == true ]] || TMP_DIRS+=("$rp")
done

LD_SNAPSHOTS=()
# Guard: with no resolved temp roots, `find` with an empty arg list would scan
# the current dir (repo root) and could delete matching paths there. Skip instead.
if [[ ${#TMP_DIRS[@]} -gt 0 ]]; then
  while IFS= read -r -d '' p; do
    LD_SNAPSHOTS+=("$p")
  done < <(find "${TMP_DIRS[@]}" -maxdepth 1 \( -name 'onchainai*.ld-snapshot' -o -name 'libonchainai*.ld-snapshot' \) -print0 2>/dev/null || true)
fi
if [[ ${#LD_SNAPSHOTS[@]} -gt 0 ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    for p in "${LD_SNAPSHOTS[@]}"; do
      du -sh "$p" 2>/dev/null || echo "[dry-run] $p"
    done
  else
    for p in "${LD_SNAPSHOTS[@]}"; do
      rm -rf "$p"
    done
    echo "removed ${#LD_SNAPSHOTS[@]} linker snapshot dir(s): ${TMP_DIRS[*]}"
  fi
fi

if [[ -n "$PLAYWRIGHT_DAYS" && -d .playwright-cli && ! -L .playwright-cli ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -print
  else
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -delete
  fi
fi