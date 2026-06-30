#!/usr/bin/env bash
# Auth/admin env readiness check — verifies required variables exist without
# printing secret values. Exits non-zero if any required key is missing.
#
# Usage:
#   ./scripts/auth-admin-readiness.sh           # check local .env
#   ./scripts/auth-admin-readiness.sh --railway  # check Railway production vars
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MODE="local"
if [[ "${1:-}" == "--railway" ]]; then
  MODE="railway"
fi

# Required keys for auth/admin to work correctly.
REQUIRED_KEYS=(
  DATABASE_URL
  SUPABASE_URL
  SUPABASE_ANON_KEY
  SUPABASE_SERVICE_KEY
  GITHUB_CLIENT_ID
  GITHUB_CLIENT_SECRET
  JWT_SECRET
  SIWX_DOMAIN
  SIWX_SESSION_TTL
)

# Optional but recommended keys (warn, not fail).
OPTIONAL_KEYS=(
  ADMIN_GITHUB_LOGINS
  GITHUB_REDIRECT_URI
  GITHUB_API_TOKEN
)

check_local() {
  if [[ -f .env ]]; then
    set -a; source .env 2>/dev/null || true; set +a
  fi

  echo "=== Local .env auth/admin readiness ==="
  local missing=0
  for key in "${REQUIRED_KEYS[@]}"; do
    val="${!key:-}"
    if [[ -z "$val" ]]; then
      echo "  MISSING: $key"
      missing=$((missing + 1))
    else
      echo "  OK: $key (value hidden)"
    fi
  done

  for key in "${OPTIONAL_KEYS[@]}"; do
    val="${!key:-}"
    if [[ -z "$val" ]]; then
      echo "  WARN (optional): $key is not set"
    else
      echo "  OK (optional): $key (value hidden)"
    fi
  done

  if [[ "$missing" -gt 0 ]]; then
    echo ""
    echo "FAIL: $missing required key(s) missing."
    exit 1
  fi
  echo ""
  echo "PASS: all required auth/admin keys are set."
}

check_railway() {
  if ! command -v railway >/dev/null 2>&1; then
    echo "Railway CLI not installed. Run: npm i -g @railway/cli" >&2
    exit 1
  fi
  if ! railway status >/dev/null 2>&1; then
    echo "No linked Railway project. Run: railway link" >&2
    exit 1
  fi

  SERVICE_NAME="${RAILWAY_SERVICE:-onchainai}"
  echo "=== Railway ($SERVICE_NAME) auth/admin readiness ==="

  # Get variable names only (railway variables prints KEY=VALUE; we check presence).
  local vars
  vars="$(railway variables 2>/dev/null || true)"

  local missing=0
  for key in "${REQUIRED_KEYS[@]}"; do
    if echo "$vars" | grep -q "^${key}="; then
      echo "  OK: $key (value hidden)"
    else
      echo "  MISSING: $key"
      missing=$((missing + 1))
    fi
  done

  for key in "${OPTIONAL_KEYS[@]}"; do
    if echo "$vars" | grep -q "^${key}="; then
      echo "  OK (optional): $key (value hidden)"
    else
      echo "  WARN (optional): $key is not set"
    fi
  done

  if [[ "$missing" -gt 0 ]]; then
    echo ""
    echo "FAIL: $missing required key(s) missing on Railway."
    exit 1
  fi
  echo ""
  echo "PASS: all required auth/admin keys are set on Railway."
}

case "$MODE" in
  local)   check_local ;;
  railway) check_railway ;;
esac
