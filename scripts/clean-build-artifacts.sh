#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DRY_RUN=false
PLAYWRIGHT_DAYS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
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

if [[ -d target ]]; then
  run cargo clean
fi

if [[ -n "$PLAYWRIGHT_DAYS" && -d .playwright-cli && ! -L .playwright-cli ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -print
  else
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -delete
  fi
fi