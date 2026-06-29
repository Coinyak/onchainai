#!/usr/bin/env bash
# Diagnose local server/cache state without mutating processes.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PORT_EXPLICIT=0
if [[ -n "${PORT+x}" ]]; then
  PORT_EXPLICIT=1
fi
PORT="${PORT:-3000}"
BASE=""
BASE_HOST="localhost"
BASE_PORT=""
PID_FILE="target/dev-server.pid"

usage() {
  cat <<'EOF'
Usage: ./scripts/local-doctor.sh [--base URL] [--port PORT]

Checks the local OnchainAI server for common stale-cache and stale-process issues:
  - listener on the selected port
  - stale target/dev-server.pid
  - coherent release bundle artifacts
  - HTML and local /pkg/* cache-control headers
  - localhost vs 127.0.0.1 browser/auth guidance

The doctor is read-only. It prints the next command instead of killing or starting
processes automatically.
EOF
}

fail_arg() {
  echo "FAIL: $*" >&2
  exit 2
}

validate_port() {
  local port="$1"
  if ! [[ "$port" =~ ^[0-9]+$ ]] ||
    (( 10#$port < 1 || 10#$port > 65535 )); then
    fail_arg "port must be a number between 1 and 65535: ${port}"
  fi
}

validate_base_url() {
  local url="$1"
  local without_scheme
  local authority
  local path_part

  case "$url" in
    http://*) ;;
    *) fail_arg "--base must be a local URL: ${url}" ;;
  esac

  without_scheme="${url#*://}"
  authority="${without_scheme%%/*}"
  path_part="${without_scheme#"$authority"}"

  if [[ -z "$authority" ]]; then
    fail_arg "--base must be a local URL: ${url}"
  fi
  if [[ -n "$path_part" && "$path_part" != "/" ]]; then
    fail_arg "--base must not include a path: ${url}"
  fi

  case "$authority" in
    localhost)
      BASE_HOST="localhost"
      BASE_PORT=""
      ;;
    localhost:*)
      BASE_HOST="localhost"
      BASE_PORT="${authority#localhost:}"
      [[ -n "$BASE_PORT" ]] || fail_arg "--base port must not be empty: ${url}"
      ;;
    127.0.0.1)
      BASE_HOST="127.0.0.1"
      BASE_PORT=""
      ;;
    127.0.0.1:*)
      BASE_HOST="127.0.0.1"
      BASE_PORT="${authority#127.0.0.1:}"
      [[ -n "$BASE_PORT" ]] || fail_arg "--base port must not be empty: ${url}"
      ;;
    \[::1\])
      BASE_HOST="::1"
      BASE_PORT=""
      ;;
    \[::1\]:*)
      BASE_HOST="::1"
      BASE_PORT="${authority#\[::1\]:}"
      [[ -n "$BASE_PORT" ]] || fail_arg "--base port must not be empty: ${url}"
      ;;
    *)
      fail_arg "--base must be a local URL: ${url}"
      ;;
  esac

  if [[ -n "$BASE_PORT" ]]; then
    validate_port "$BASE_PORT"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      [[ $# -ge 2 ]] || {
        echo "FAIL: --base requires a URL" >&2
        exit 2
      }
      BASE="$2"
      shift 2
      ;;
    --port)
      [[ $# -ge 2 ]] || {
        echo "FAIL: --port requires a port" >&2
        exit 2
      }
      PORT="$2"
      PORT_EXPLICIT=1
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "FAIL: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

validate_port "$PORT"

if [[ -z "$BASE" ]]; then
  BASE="http://localhost:${PORT}"
else
  BASE="${BASE%/}"
  validate_base_url "$BASE"
  if [[ -n "$BASE_PORT" ]]; then
    if (( PORT_EXPLICIT )) && [[ "$BASE_PORT" != "$PORT" ]]; then
      fail_arg "--base port ${BASE_PORT} does not match explicit port ${PORT}"
    fi
    PORT="$BASE_PORT"
  else
    BASE="${BASE}:${PORT}"
  fi
fi

lowercase() {
  tr '[:upper:]' '[:lower:]'
}

has_cache_token() {
  local cache
  local token
  local directive
  local directives
  cache="$(printf '%s' "$1" | lowercase)"
  token="$(printf '%s' "$2" | lowercase)"

  IFS=',' read -r -a directives <<< "$cache"
  for directive in "${directives[@]}"; do
    directive="${directive#"${directive%%[![:space:]]*}"}"
    directive="${directive%"${directive##*[![:space:]]}"}"
    if [[ "$directive" == "$token" ]]; then
      return 0
    fi
  done

  return 1
}

fetch_headers() {
  local path="$1"
  local out="$2"
  curl -sS -I --max-time 5 "${BASE}${path}" >"$out"
}

tmp_file() {
  mktemp -t onchainai-local-doctor.XXXXXX
}

header_status() {
  awk '
    {
      line = $0
      sub(/\r$/, "", line)
      if (tolower(line) ~ /^http\//) {
        print $2
        exit
      }
    }
  ' "$1"
}

cache_control() {
  awk '
    {
      line = $0
      sub(/\r$/, "", line)
      if (tolower(line) ~ /^cache-control:/) {
        sub(/^[^:]*:[[:space:]]*/, "", line)
        print line
        exit
      }
    }
  ' "$1"
}

LISTENERS=""
LSOF_AVAILABLE=1

collect_listener_pids() {
  local out
  local status

  if ! command -v lsof >/dev/null 2>&1; then
    LSOF_AVAILABLE=0
    return 0
  fi

  set +e
  out="$(lsof -nP -iTCP:"${PORT}" -sTCP:LISTEN -t 2>/dev/null)"
  status=$?
  set -e

  if (( status > 1 )); then
    LSOF_AVAILABLE=0
    return 0
  fi

  if [[ -n "$out" ]]; then
    LISTENERS="$(printf '%s\n' "$out" | paste -sd ',' -)"
  fi
}

check_stale_pid() {
  [[ -f "$PID_FILE" ]] || return 0

  local pid
  pid="$(head -n 1 "$PID_FILE" | tr -cd '0-9')"
  [[ -n "$pid" ]] || return 0

  if ! ps -p "$pid" >/dev/null 2>&1; then
    echo "WARN: ${PID_FILE} points to a dead process: ${pid}"
  fi
}

check_bundle() {
  if [[ ! -x ./scripts/verify-bundle.sh ]]; then
    echo "WARN: ./scripts/verify-bundle.sh is missing or not executable"
    return 0
  fi

  local out
  out="$(tmp_file)"
  if ./scripts/verify-bundle.sh >"$out" 2>&1; then
    echo "OK: bundle artifacts look coherent"
    rm -f "$out"
    return 0
  fi

  echo "FAIL: ./scripts/verify-bundle.sh failed"
  sed 's/^/  /' "$out"
  rm -f "$out"
  echo "Next: run ./scripts/dev-watch.sh while editing, or ./scripts/restart-dev.sh --foreground after a release build."
  return 1
}

check_html_cache() {
  local path="/"
  local out
  local code
  local cache

  out="$(tmp_file)"
  if ! fetch_headers "$path" "$out"; then
    echo "FAIL: could not fetch ${BASE}${path}"
    rm -f "$out"
    return 1
  fi

  code="$(header_status "$out")"
  cache="$(cache_control "$out")"
  rm -f "$out"

  if [[ "$code" != "200" ]]; then
    echo "FAIL: / returned HTTP ${code:-unknown}"
    return 1
  fi

  if ! has_cache_token "$cache" "private" ||
    ! has_cache_token "$cache" "no-cache" ||
    ! has_cache_token "$cache" "max-age=0" ||
    ! has_cache_token "$cache" "must-revalidate"; then
    echo "FAIL: / cache-control is ${cache:-<missing>}"
    echo "Expected: private, no-cache, max-age=0, must-revalidate"
    return 1
  fi

  echo "OK: / cache-control is ${cache}"
}

check_pkg_cache() {
  local path="$1"
  local out
  local code
  local cache

  out="$(tmp_file)"
  if ! fetch_headers "$path" "$out"; then
    echo "FAIL: could not fetch ${BASE}${path}"
    rm -f "$out"
    return 1
  fi

  code="$(header_status "$out")"
  cache="$(cache_control "$out")"
  rm -f "$out"

  if [[ "$code" != "200" ]]; then
    echo "FAIL: ${path} returned HTTP ${code:-unknown}"
    return 1
  fi

  if ! has_cache_token "$cache" "no-store"; then
    echo "FAIL: ${path} cache-control is ${cache:-<missing>}"
    echo "Expected local dev pkg assets to be no-store"
    return 1
  fi

  echo "OK: ${path} cache-control is ${cache}"
}

echo "Local doctor: ${BASE}"

if [[ "$BASE_HOST" == "127.0.0.1" || "$BASE_HOST" == "::1" ]]; then
  echo "WARN: use http://localhost:${PORT} for browser/auth checks when possible"
fi

failures=0
check_stale_pid

collect_listener_pids
if (( LSOF_AVAILABLE )); then
  if [[ -z "$LISTENERS" ]]; then
    echo "FAIL: no process is listening on port ${PORT}"
    echo "Next: start the local loop with ./scripts/dev-watch.sh"
    echo "      For release-mode parity, run ./scripts/restart-dev.sh --foreground in a terminal."
    exit 1
  fi
  echo "OK: port ${PORT} has listener pid(s): ${LISTENERS}"
else
  echo "WARN: lsof not available; using HTTP reachability checks instead"
fi

check_bundle || failures=$((failures + 1))
check_html_cache || failures=$((failures + 1))
check_pkg_cache "/pkg/onchainai.js" || failures=$((failures + 1))
check_pkg_cache "/pkg/onchainai.wasm" || failures=$((failures + 1))
check_pkg_cache "/pkg/onchainai.css" || failures=$((failures + 1))

if (( failures > 0 )); then
  echo "Next: run ./scripts/dev-watch.sh while editing, or ./scripts/restart-dev.sh --foreground after a release build."
  exit 1
fi

echo "OK: local server/cache looks healthy for ${BASE}"
echo "Next: keep ./scripts/dev-watch.sh running while editing."
