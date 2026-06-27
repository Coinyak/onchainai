#!/usr/bin/env bash
# Verify release binary and WASM/JS client bundle were built together (coherent mtimes).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BINARY="target/release/onchainai"
WASM="target/site/pkg/onchainai.wasm"
JS="target/site/pkg/onchainai.js"
CSS="style/output.css"
PKG_BG="target/site/pkg/onchainai_bg.wasm"
MAX_SKEW_SEC="${ONCHAINAI_BUNDLE_MAX_SKEW_SEC:-180}"

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

for artifact in "$BINARY" "$WASM" "$JS" "$CSS"; do
  if [[ ! -e "$artifact" ]]; then
    echo "  MISSING  ${artifact}"
    status="FAIL"
    reasons+=("missing:${artifact}")
    continue
  fi
  echo "  $(file_mtime_human "$artifact")  ${artifact}"
done

if [[ ! -e "$WASM" ]]; then
  reasons+=("wasm missing")
elif [[ ! -e "$BINARY" ]]; then
  status="FAIL"
  reasons+=("binary missing")
elif [[ ! -e "$JS" ]]; then
  status="FAIL"
  reasons+=("js missing")
elif [[ ! -e "$CSS" ]]; then
  status="FAIL"
  reasons+=("css missing")
else
  if [[ ! -e "$PKG_BG" ]]; then
    echo "Creating ${PKG_BG} symlink → onchainai.wasm"
    ln -sf onchainai.wasm "$PKG_BG"
  fi

  bin_mtime="$(file_mtime "$BINARY")"
  wasm_mtime="$(file_mtime "$WASM")"
  js_mtime="$(file_mtime "$JS")"
  css_mtime="$(file_mtime "$CSS")"

  wasm_skew=$((wasm_mtime - bin_mtime))
  js_skew=$((js_mtime - bin_mtime))
  css_skew=$((css_mtime - bin_mtime))

  abs_wasm_skew="${wasm_skew#-}"
  abs_js_skew="${js_skew#-}"
  abs_css_skew="${css_skew#-}"

  echo ""
  echo "Skew vs binary (non-zero → mixed-build risk):"
  echo "  wasm: ${wasm_skew}s"
  echo "  js:   ${js_skew}s"
  echo "  css:  ${css_skew}s"
  echo "  tolerance: ±${MAX_SKEW_SEC}s"

  if [[ "$abs_wasm_skew" -gt "$MAX_SKEW_SEC" ]]; then
    status="FAIL"
    if [[ "$wasm_skew" -gt 0 ]]; then
      reasons+=("binary older than wasm by ${wasm_skew}s (>${MAX_SKEW_SEC}s)")
    else
      reasons+=("binary newer than wasm by ${abs_wasm_skew}s (>${MAX_SKEW_SEC}s) — partial server rebuild?")
    fi
  fi
  if [[ "$abs_js_skew" -gt "$MAX_SKEW_SEC" ]]; then
    status="FAIL"
    if [[ "$js_skew" -gt 0 ]]; then
      reasons+=("binary older than js by ${js_skew}s (>${MAX_SKEW_SEC}s)")
    else
      reasons+=("binary newer than js by ${abs_js_skew}s (>${MAX_SKEW_SEC}s) — partial server rebuild?")
    fi
  fi
  if [[ "$abs_css_skew" -gt "$MAX_SKEW_SEC" ]]; then
    status="FAIL"
    if [[ "$css_skew" -gt 0 ]]; then
      reasons+=("binary older than css by ${css_skew}s (>${MAX_SKEW_SEC}s)")
    else
      reasons+=("binary newer than css by ${abs_css_skew}s (>${MAX_SKEW_SEC}s)")
    fi
  fi
fi

echo ""
if [[ "$status" == "PASS" ]]; then
  echo "VERIFY BUNDLE PASS (artifacts coherent)"
  exit 0
fi

echo "VERIFY BUNDLE FAIL"
for r in "${reasons[@]}"; do
  echo "  - ${r}" >&2
done
echo "Run: ./scripts/release-build.sh" >&2
exit 1
