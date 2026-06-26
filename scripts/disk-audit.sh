#!/usr/bin/env bash
# Monthly disk audit — see docs/DISK_MAINTENANCE.md § "Quick audit (monthly)".
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOME_DIR="${HOME:-/Users/love}"
DATA_VOL="/System/Volumes/Data"
FACTORY_LOG="${HOME_DIR}/.factory/logs/droid-log-single.log"
FACTORY_WARN_MB="${ONCHAINAI_FACTORY_LOG_WARN_MB:-500}"
VARF="${ONCHAINAI_VARFOLDERS:-}"

echo "=== OnchainAI disk audit $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
echo

echo "--- Filesystem (target: free >= 25GB for builds) ---"
df -h "${DATA_VOL}" 2>/dev/null || df -h .
free_kb="$(df -Pk "${DATA_VOL}" 2>/dev/null | awk 'NR==2 {print $4}' || df -Pk . | awk 'NR==2 {print $4}')"
free_gb="$((free_kb / 1024 / 1024))"
echo "free_disk_gb=${free_gb}"
if (( free_gb < 25 )); then
  echo "WARN: free disk below 25GB — run clean-build-artifacts.sh before release build" >&2
fi
echo

echo "--- Home + agent state ---"
du -sh "${HOME_DIR}" "${HOME_DIR}/.factory/logs" "${HOME_DIR}/.codex" 2>/dev/null || true
if [[ -f "${HOME_DIR}/.hermes/state.db" ]]; then
  du -sh "${HOME_DIR}/.hermes/state.db"
else
  echo "(no ~/.hermes/state.db)"
fi
echo

echo "--- Factory log (truncate when > ${FACTORY_WARN_MB}MB) ---"
if [[ -f "${FACTORY_LOG}" ]]; then
  log_kb="$(du -sk "${FACTORY_LOG}" | awk '{print $1}')"
  log_mb="$((log_kb / 1024))"
  echo "droid-log-single.log: ${log_mb}MB"
  if (( log_mb > FACTORY_WARN_MB )); then
    echo "WARN: factory log exceeds ${FACTORY_WARN_MB}MB — consider: truncate -s 0 ${FACTORY_LOG}" >&2
  fi
else
  echo "(no factory log)"
fi
echo

echo "--- macOS var/folders code_sign_clone (quarterly cleanup) ---"
if [[ -z "${VARF}" ]]; then
  VARF="$(getconf DARWIN_USER_TEMP_DIR 2>/dev/null | sed 's|/T/$||' | sed 's|/T||' || true)"
fi
if [[ -n "${VARF}" && -d "${VARF}/X" ]]; then
  du -sh "${VARF}/X/"*code_sign_clone 2>/dev/null || echo "(no code_sign_clone dirs)"
else
  echo "(var/folders path unavailable — set ONCHAINAI_VARFOLDERS)"
fi
echo

echo "--- Largest Rust target/ dirs under home ---"
find "${HOME_DIR}" -name target -type d -prune -exec du -sh {} \; 2>/dev/null \
  | sort -hr | head -5 || true
echo

echo "--- OnchainAI project sizes (target <= 35GB) ---"
cd "${ROOT}"
du -sh target target/site target/release target/debug 2>/dev/null || echo "(no target/)"
./scripts/disk-guard.sh || true

echo
echo "=== audit complete ==="