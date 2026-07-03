# Shared curl helpers for smoke-test*.sh (source only; not executable).
smoke_fail() {
  echo "SMOKE FAIL: $*" >&2
  exit 1
}

smoke_check_get() {
  local base="$1"
  local path="$2"
  local body
  body="$(mktemp)"
  local code
  code="$(curl -sS -L -o "$body" -w "%{http_code}" "${base}${path}")" \
    || smoke_fail "GET ${path} curl failed"
  [[ "$code" == "200" ]] || smoke_fail "GET ${path} returned ${code}"
  if grep -qiE "error deserializing|missing field filters|panic|not found: /pkg" "$body"; then
    echo "---- body excerpt ----" >&2
    head -80 "$body" >&2
    smoke_fail "GET ${path} contains app error"
  fi
  cat "$body"
  rm -f "$body"
}

smoke_body_has() {
  local body="$1"
  local needle="$2"
  printf '%s' "$body" | grep -Fq "$needle"
}

smoke_detect_stack() {
  local base="$1"
  local probe
  probe="$(mktemp)"
  local code
  code="$(curl -sS -L -o "$probe" -w "%{http_code}" "${base}/" 2>/dev/null || echo "000")"
  if [[ "$code" != "200" ]]; then
    rm -f "$probe"
    echo "unknown"
    return 0
  fi
  if grep -q '_next/static' "$probe"; then
    rm -f "$probe"
    echo "frontend"
    return 0
  fi
  if grep -qE 'sidebar-brand|/pkg/onchainai' "$probe"; then
    rm -f "$probe"
    echo "leptos"
    return 0
  fi
  rm -f "$probe"
  echo "unknown"
}