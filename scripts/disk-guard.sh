#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Always sweep stray linker snapshots first (no-op on Linux / when none exist).
# They accumulate in the temp dir independently of target/ size — each macOS
# dylib link can leave a multi-GB *.ld-snapshot — so a target-size threshold
# never catches them. Do this before measuring so free space reflects the sweep.
"${ROOT}/scripts/clean-build-artifacts.sh" --snapshots-only >&2 || true

# Integer GB (floored) — 24.9GB reports as 24 and fails the 25GB default.
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

# Self-heal: when over a threshold, drop incremental caches automatically
# (keeps compiled deps so the next build stays fast) and re-measure before
# failing. Lets any agent build without a human babysitting disk. Disable with
# ONCHAINAI_DISK_GUARD_AUTOCLEAN=0.
if (( free_gb < MIN_FREE_GB )) || (( target_gb > MAX_TARGET_GB )); then
  if [[ "${ONCHAINAI_DISK_GUARD_AUTOCLEAN:-1}" == "1" ]]; then
    echo "disk-guard: over threshold; auto-cleaning incremental caches" >&2
    "${ROOT}/scripts/clean-build-artifacts.sh" --incremental-only >&2 || true
    free_kb="$(df -Pk . | awk 'NR==2 {print $4}')"
    free_gb="$((free_kb / 1024 / 1024))"
    target_gb="0"
    if [[ -d target ]]; then
      target_kb="$(du -sk target | awk '{print $1}')"
      target_gb="$((target_kb / 1024 / 1024))"
    fi
    echo "disk-guard: after auto-clean free_disk_gb=${free_gb} target_gb=${target_gb}" >&2
  fi
fi

if (( free_gb < MIN_FREE_GB )); then
  echo "ERROR: free disk ${free_gb}GB is below ${MIN_FREE_GB}GB" >&2
  echo "Try: ./scripts/clean-build-artifacts.sh --incremental-only" >&2
  echo "Then: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  echo "Also check /tmp/onchainai*.ld-snapshot (multi-GB linker failures on macOS)" >&2
  exit 1
fi

if (( target_gb > MAX_TARGET_GB )); then
  echo "ERROR: target ${target_gb}GB exceeds ${MAX_TARGET_GB}GB" >&2
  echo "Try: ./scripts/clean-build-artifacts.sh --incremental-only" >&2
  echo "Then: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  exit 1
fi