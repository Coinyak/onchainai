#!/usr/bin/env bash
# Optional lighter cleanup after builds — trims debug artifacts without full cargo clean.
#
# 1. Removes files in target/debug/deps older than 7 days (stale incremental deps).
# 2. If target/ still exceeds 8GB, keeps only wasm32 + release and deletes debug/.
#
# Run manually after heavy build sessions:
#   ./scripts/post-build-clean.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET_MAX_GB=8
DEPS_MAX_AGE_DAYS=7

target_size_kb() {
  if [[ -d target ]]; then
    du -sk target 2>/dev/null | awk '{print $1}'
  else
    echo 0
  fi
}

target_size_human() {
  if [[ -d target ]]; then
    du -sh target 2>/dev/null | awk '{print $1}'
  else
    echo "0B"
  fi
}

kb_to_gb() {
  awk -v kb="$1" 'BEGIN { printf "%.2f", kb / 1024 / 1024 }'
}

echo "=== post-build-clean ==="
BEFORE_KB="$(target_size_kb)"
echo "target/ size before: $(target_size_human) ($(kb_to_gb "$BEFORE_KB")GB)"

# Light cleanup: stale debug/deps artifacts
if [[ -d target/debug/deps ]]; then
  REMOVED=0
  while IFS= read -r -d '' f; do
    rm -f "$f"
    REMOVED=$((REMOVED + 1))
  done < <(find target/debug/deps -type f -mtime +"${DEPS_MAX_AGE_DAYS}" -print0 2>/dev/null || true)
  echo "Removed ${REMOVED} file(s) in target/debug/deps older than ${DEPS_MAX_AGE_DAYS} days."
else
  echo "No target/debug/deps directory — skipping stale deps cleanup."
fi

AFTER_LIGHT_KB="$(target_size_kb)"
TARGET_MAX_KB=$((TARGET_MAX_GB * 1024 * 1024))

if (( AFTER_LIGHT_KB > TARGET_MAX_KB )); then
  echo ""
  echo "target/ exceeds ${TARGET_MAX_GB}GB ($(kb_to_gb "$AFTER_LIGHT_KB")GB). Pruning debug/ (keeping wasm32-unknown-unknown + release)..."
  if [[ -d target/debug ]]; then
    rm -rf target/debug
    echo "Deleted target/debug/"
  fi
  # Remove other non-essential debug-adjacent caches while preserving release + wasm
  for dir in target/tmp target/incremental; do
    if [[ -d "$dir" ]]; then
      rm -rf "$dir"
      echo "Deleted ${dir}/"
    fi
  done
else
  echo "target/ within ${TARGET_MAX_GB}GB after light cleanup — skipping aggressive prune."
fi

AFTER_KB="$(target_size_kb)"
echo ""
echo "target/ size after: $(target_size_human) ($(kb_to_gb "$AFTER_KB")GB)"
echo "Done."