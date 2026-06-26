#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MIN_FREE_GB="${ONCHAINAI_MIN_FREE_GB:-25}"
MAX_TARGET_GB="${ONCHAINAI_MAX_TARGET_GB:-35}"

free_kb="$(df -Pk . | awk 'NR==2 {print $4}')"
free_gb="$((free_kb / 1024 / 1024))"
target_gb="0"
if [[ -d target ]]; then
  target_kb="$(du -sk target | awk '{print $1}')"
  target_gb="$((target_kb / 1024 / 1024))"
fi

echo "free_disk_gb=${free_gb}"
echo "target_gb=${target_gb}"
du -sh target target/site .playwright-cli 2>/dev/null || true

if [[ "${ONCHAINAI_DISK_GUARD_FORCE:-0}" == "1" ]]; then
  echo "ONCHAINAI_DISK_GUARD_FORCE=1 set; continuing"
  exit 0
fi

if (( free_gb < MIN_FREE_GB )); then
  echo "ERROR: free disk ${free_gb}GB is below ${MIN_FREE_GB}GB" >&2
  echo "Run: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  echo "Also check /tmp/onchainai*.ld-snapshot (multi-GB linker failures on macOS)" >&2
  exit 1
fi

if (( target_gb > MAX_TARGET_GB )); then
  echo "ERROR: target ${target_gb}GB exceeds ${MAX_TARGET_GB}GB" >&2
  echo "Run: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  exit 1
fi