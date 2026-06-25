#!/usr/bin/env bash
# Set www.onchain-ai.xyz DNS records on GoDaddy via API (when web UI shows Akamai errors).
#
# 1. https://developer.godaddy.com/keys → Create API Key (Production)
# 2. export GODADDY_API_KEY=... GODADDY_API_SECRET=...
# 3. ./scripts/configure-godaddy-dns.sh
set -euo pipefail

: "${GODADDY_API_KEY:?Set GODADDY_API_KEY from https://developer.godaddy.com/keys}"
: "${GODADDY_API_SECRET:?Set GODADDY_API_SECRET}"

DOMAIN="www.onchain-ai.xyz"
AUTH="sso-key ${GODADDY_API_KEY}:${GODADDY_API_SECRET}"
BASE="https://api.godaddy.com/v1/domains/${DOMAIN}"

echo "Fetching current DNS records..."
/usr/bin/curl -s -H "Authorization: ${AUTH}" "${BASE}/records" | /usr/bin/python3 -m json.tool

echo ""
echo "Resetting nameservers to GoDaddy default (leaves Afternic parking)..."
NS_JSON=$(/usr/bin/curl -s -H "Authorization: ${AUTH}" "${BASE}" | /usr/bin/python3 -c "
import sys, json
d = json.load(sys.stdin)
ns = d.get('nameServers') or []
if any('afternic' in n.lower() for n in ns):
    print(json.dumps({'nameServers': ['ns07.domaincontrol.com', 'ns08.domaincontrol.com']}))
else:
    print('')
" 2>/dev/null || true)
if [[ -n "${NS_JSON}" ]]; then
  /usr/bin/curl -s -X PATCH -H "Authorization: ${AUTH}" -H "Content-Type: application/json" \
    -d "${NS_JSON}" "${BASE}"
  echo "Nameservers updated; wait 5-10 min before record changes."
fi

echo ""
echo "Removing parking A record @ if present..."
/usr/bin/curl -s -X DELETE -H "Authorization: ${AUTH}" "${BASE}/records/A/@" || true

echo ""
echo "Setting CNAME @ -> 8k7un69e.up.railway.app ..."
HTTP=$(/usr/bin/curl -s -o /tmp/gd-cname.json -w "%{http_code}" -X PUT \
  -H "Authorization: ${AUTH}" -H "Content-Type: application/json" \
  -d '[{"data":"8k7un69e.up.railway.app","ttl":600}]' \
  "${BASE}/records/CNAME/@")
echo "CNAME HTTP ${HTTP}"; cat /tmp/gd-cname.json 2>/dev/null; echo

echo "Setting TXT _railway-verify ..."
HTTP=$(/usr/bin/curl -s -o /tmp/gd-txt.json -w "%{http_code}" -X PUT \
  -H "Authorization: ${AUTH}" -H "Content-Type: application/json" \
  -d '[{"data":"railway-verify=bb3c93ae6b6270a82c3accb6b5deaa332c872e3d9dfaf0f14c7b342495163705","ttl":600}]' \
  "${BASE}/records/TXT/_railway-verify")
echo "TXT HTTP ${HTTP}"; cat /tmp/gd-txt.json 2>/dev/null; echo

echo ""
echo "Done. Verify:"
dig +short "${DOMAIN}" CNAME
dig +short "${DOMAIN}" A
dig +short "_railway-verify.${DOMAIN}" TXT