#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Always sweep stray linker snapshots first (no-op on Linux / when none exist).
# They accumulate in the temp dir independently of target/ size — each macOS
# dylib link can leave a multi-GB *.ld-snapshot — so a target-size threshold
# never catches them. Do this before measuring so free space reflects the sweep.
# Best-effort, but surface failures (do not silently mask) so a broken sweep is
# debuggable rather than hidden behind a still-low-disk threshold error.
"${ROOT}/scripts/clean-build-artifacts.sh" --snapshots-only >&2 \
  || echo "WARN: snapshot sweep failed (exit $?); continuing disk guard best-effort" >&2

# Integer GB (floored) — 24.9GB reports as 24 and fails the 25GB default.
MIN_FREE_GB="${ONCHAINAI_MIN_FREE_GB:-25}"
MAX_TARGET_GB="${ONCHAINAI_MAX_TARGET_GB:-35}"
STALE_MAIN_CRATE_PRUNE_GB="${ONCHAINAI_STALE_MAIN_CRATE_PRUNE_GB:-16}"
STALE_MAIN_CRATE_KEEP="${ONCHAINAI_STALE_MAIN_CRATE_KEEP:-3}"

for numeric_var in MIN_FREE_GB MAX_TARGET_GB STALE_MAIN_CRATE_PRUNE_GB STALE_MAIN_CRATE_KEEP; do
  numeric_value="${!numeric_var}"
  case "$numeric_value" in
    ''|*[!0-9]*)
      echo "ERROR: ${numeric_var} must be a non-negative integer (got: ${numeric_value})" >&2
      exit 2
      ;;
  esac
done
if (( STALE_MAIN_CRATE_KEEP < 1 )); then
  echo "ERROR: ONCHAINAI_STALE_MAIN_CRATE_KEEP must be at least 1" >&2
  exit 2
fi

measure_usage() {
  free_kb="$(df -Pk . | awk 'NR==2 {print $4}')"
  free_gb="$((free_kb / 1024 / 1024))"
  target_gb="0"
  if [[ -d target ]]; then
    target_kb="$(du -sk target | awk '{print $1}')"
    target_gb="$((target_kb / 1024 / 1024))"
  fi
}

measure_usage

echo "free_disk_gb=${free_gb}"
echo "target_gb=${target_gb}"
du -sh target target/site .playwright-cli 2>/dev/null || true

if [[ "${ONCHAINAI_DISK_GUARD_FORCE:-0}" == "1" ]]; then
  echo "ONCHAINAI_DISK_GUARD_FORCE=1 set; continuing"
  exit 0
fi

# Self-heal: when target/ starts getting large, first drop stale hashed
# onchainai/libonchainai debug artifacts. This keeps third-party compiled deps
# and the newest local crate artifact groups, which avoids turning every build
# into a cold build while preventing repeated debug builds from piling up.
if (( target_gb > STALE_MAIN_CRATE_PRUNE_GB )); then
  if [[ "${ONCHAINAI_DISK_GUARD_AUTOCLEAN:-1}" == "1" ]]; then
    echo "disk-guard: target ${target_gb}GB exceeds stale artifact prune threshold ${STALE_MAIN_CRATE_PRUNE_GB}GB; pruning stale main-crate artifacts" >&2
    "${ROOT}/scripts/clean-build-artifacts.sh" \
      --stale-main-crate \
      --stale-main-crate-keep "${STALE_MAIN_CRATE_KEEP}" >&2 || true
    measure_usage
    echo "disk-guard: after stale prune free_disk_gb=${free_gb} target_gb=${target_gb}" >&2
  fi
fi

# If still over a hard threshold, drop incremental caches automatically (keeps
# compiled deps so the next build stays reasonably fast) and re-measure before
# failing. Disable with ONCHAINAI_DISK_GUARD_AUTOCLEAN=0.
if (( free_gb < MIN_FREE_GB )) || (( target_gb > MAX_TARGET_GB )); then
  if [[ "${ONCHAINAI_DISK_GUARD_AUTOCLEAN:-1}" == "1" ]]; then
    echo "disk-guard: over threshold; auto-cleaning incremental caches" >&2
    "${ROOT}/scripts/clean-build-artifacts.sh" --incremental-only >&2 || true
    measure_usage
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
