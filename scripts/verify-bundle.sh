#!/usr/bin/env bash
# Verify release API binary and Next.js build exist and were built together.
# No WASM/pkg checks — Leptos was removed; UI is frontend/.next on Vercel.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BINARY="target/release/onchainai"
NEXT_BUILD="frontend/.next/BUILD_ID"
MAX_SKEW_SEC="${ONCHAINAI_BUNDLE_MAX_SKEW_SEC:-600}"

fail() {
  echo "VERIFY BUNDLE FAIL: $*" >&2
  exit 1
}

file_mtime() {
  local path="$1"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    stat -f '%m' "$path"
  else
    stat -c '%Y' "$path"
  fi
}

file_mtime_human() {
  local path="$1"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' "$path"
  else
    stat -c '%y' "$path" | cut -d. -f1
  fi
}

echo "Bundle artifact timestamps:"
status="PASS"
reasons=()

for artifact in "$BINARY" "$NEXT_BUILD"; do
  if [[ ! -e "$artifact" ]]; then
    echo "  MISSING  ${artifact}"
    status="FAIL"
    reasons+=("missing:${artifact}")
    continue
  fi
  echo "  $(file_mtime_human "$artifact")  ${artifact}"
done

if [[ ! -x "$BINARY" ]]; then
  status="FAIL"
  reasons+=("api binary missing or not executable")
elif [[ ! -f "$NEXT_BUILD" ]]; then
  status="FAIL"
  reasons+=("next build missing (run: cd frontend && npm run build)")
else
  bin_mtime="$(file_mtime "$BINARY")"
  next_mtime="$(file_mtime "$NEXT_BUILD")"
  skew=$((next_mtime - bin_mtime))
  abs_skew="${skew#-}"

  echo ""
  echo "Skew vs API binary (non-zero → mixed-build risk):"
  echo "  next: ${skew}s"
  echo "  tolerance: ±${MAX_SKEW_SEC}s"

  if [[ "$abs_skew" -gt "$MAX_SKEW_SEC" ]]; then
    status="FAIL"
    if [[ "$skew" -gt 0 ]]; then
      reasons+=("api binary older than next build by ${skew}s (>${MAX_SKEW_SEC}s)")
    else
      reasons+=("api binary newer than next build by ${abs_skew}s (>${MAX_SKEW_SEC}s) — partial rebuild?")
    fi
  fi
fi

echo ""
if [[ "$status" == "PASS" ]]; then
  echo "VERIFY BUNDLE PASS (API + Next.js artifacts coherent)"
  exit 0
fi

echo "VERIFY BUNDLE FAIL"
for r in "${reasons[@]}"; do
  echo "  - ${r}" >&2
done
echo "Run: ./scripts/release-build.sh" >&2
exit 1