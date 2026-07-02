#!/usr/bin/env bash
# Point www.onchain-ai.xyz DNS at Vercel (GoDaddy API).
#
# Prerequisites:
#   export GODADDY_API_KEY=...
#   export GODADDY_API_SECRET=...
#   Keys: https://developer.godaddy.com/keys (Production; domain must be in account)
#
# Usage:
#   ./scripts/godaddy-dns-vercel.sh
#   DNS_RECORD_TYPE=A ./scripts/godaddy-dns-vercel.sh
#   VERCEL_CNAME_TARGET=69cb859cfbfc4298.vercel-dns-017.com ./scripts/godaddy-dns-vercel.sh
set -euo pipefail

DOMAIN="${GODADDY_DOMAIN:-onchain-ai.xyz}"
HOST="${GODADDY_HOST:-www}"
RECORD_TYPE="${DNS_RECORD_TYPE:-A}"
A_TARGET="${VERCEL_A_TARGET:-76.76.21.21}"
CNAME_TARGET="${VERCEL_CNAME_TARGET:-69cb859cfbfc4298.vercel-dns-017.com}"

: "${GODADDY_API_KEY:?Set GODADDY_API_KEY}"
: "${GODADDY_API_SECRET:?Set GODADDY_API_SECRET}"

API="https://api.godaddy.com/v1/domains/${DOMAIN}/records"
AUTH="sso-key ${GODADDY_API_KEY}:${GODADDY_API_SECRET}"

echo "Fetching existing ${HOST} records..."
existing="$(curl -sS -X GET "${API}" \
  -H "Authorization: ${AUTH}" \
  -H "Content-Type: application/json")"

echo "${existing}" | python3 -c "
import sys, json
host = '${HOST}'
for r in json.load(sys.stdin):
    if r.get('name') == host:
        print(json.dumps(r, indent=2))
" 2>/dev/null || echo "${existing}"

if [[ "${RECORD_TYPE}" == "A" ]]; then
  target="${A_TARGET}"
  new_type="A"
  echo "Replacing ${HOST}.${DOMAIN} → A ${A_TARGET} ..."
  for old_type in CNAME A AAAA; do
    curl -sS -o /dev/null -w "DELETE ${old_type}/${HOST} HTTP %{http_code}\n" \
      -X DELETE -H "Authorization: ${AUTH}" "${API}/${old_type}/${HOST}" || true
  done
else
  target="${CNAME_TARGET}"
  new_type="CNAME"
  echo "Replacing ${HOST}.${DOMAIN} → CNAME ${CNAME_TARGET} ..."
  for old_type in CNAME A AAAA; do
    curl -sS -o /dev/null -w "DELETE ${old_type}/${HOST} HTTP %{http_code}\n" \
      -X DELETE -H "Authorization: ${AUTH}" "${API}/${old_type}/${HOST}" || true
  done
fi

curl -sS -X PUT "${API}/${new_type}/${HOST}" \
  -H "Authorization: ${AUTH}" \
  -H "Content-Type: application/json" \
  -d "[{\"data\":\"${target}\",\"ttl\":600}]"

echo ""
echo "Done. Verify (propagation may take a few minutes):"
echo "  dig +short www.${DOMAIN}"
echo "  npx vercel domains verify www.${DOMAIN} --scope onchain-ai"