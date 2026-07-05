#!/usr/bin/env bash
# Fail when vendor-orgs.json lists crawl:true orgs that do not exist on GitHub.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/scripts/vendor-orgs.json"

fail=0
log() { printf '%s\n' "$*"; }
fail_check() { log "FAIL: $*"; fail=1; }
pass_check() { log "PASS: $*"; }

if [[ ! -f "$MANIFEST" ]]; then
  fail_check "missing $MANIFEST"
  exit 1
fi

while IFS=$'\t' read -r org; do
  [[ -z "$org" ]] && continue
  code=$(curl -sS -o /dev/null -w "%{http_code}" \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: onchainai-manifest-check" \
    "https://api.github.com/orgs/${org}" || echo "000")
  if [[ "$code" == "200" ]]; then
    pass_check "GitHub org exists: $org"
  else
    fail_check "GitHub org $org returned HTTP $code (crawl:true must be a real org)"
  fi
  sleep 0.2
done < <(python3 - "$MANIFEST" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
for entry in data.get("orgs", []):
    if entry.get("crawl"):
        print(entry["github"])
PY
)

if [[ $fail -ne 0 ]]; then
  log ""
  log "VENDOR-ORGS MANIFEST: FAIL"
  exit 1
fi

log ""
log "VENDOR-ORGS MANIFEST: PASS"
exit 0