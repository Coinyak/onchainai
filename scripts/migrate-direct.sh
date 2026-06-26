#!/usr/bin/env bash
# Apply sqlx migrations via Supabase direct connection (avoids pooler session limits).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

: "${DATABASE_URL:?Set DATABASE_URL in .env}"

PROJECT_REF="${SUPABASE_PROJECT_REF:-puvxrdsgexjxvgfiepua}"
DIRECT_HOST="db.${PROJECT_REF}.supabase.co"

# Rewrite pooler host → direct host, keep credentials unchanged.
export DIRECT_HOST
export DATABASE_URL="$(
  python3 <<'PY'
import os, urllib.parse
u = urllib.parse.urlparse(os.environ["DATABASE_URL"])
host = os.environ["DIRECT_HOST"]
direct = u._replace(netloc=f"{u.username}:{u.password}@{host}:5432")
print(urllib.parse.urlunparse(direct))
PY
)"

echo "Running sqlx migrate via ${DIRECT_HOST}:5432 ..."
sqlx migrate run
echo "Migrations applied."