#!/usr/bin/env bash
# Verify OPERATOR_GUIDE / MVP_DESIGN crawler docs match wired CRAWLER_SOURCE_DEFS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ADMIN_RS="src/server/functions/crawler_admin.rs"
SCHEDULER_RS="src/crawler/scheduler.rs"
OPERATOR_GUIDE="docs/OPERATOR_GUIDE.md"
MVP_DESIGN="docs/MVP_DESIGN.md"

fail=0

log() { printf '%s\n' "$*"; }
fail_check() { log "FAIL: $*"; fail=1; }
pass_check() { log "PASS: $*"; }

# --- Extract discovery source names from CRAWLER_SOURCE_DEFS (shipped code) ---
CODE_SOURCES=()
while IFS= read -r line; do
  CODE_SOURCES+=("$line")
done < <(
  awk '/pub\(crate\) const CRAWLER_SOURCE_DEFS/,/\];/' "$ADMIN_RS" \
    | grep -oE '"[a-z0-9_-]+"' \
    | tr -d '"' \
    | sort
)

if [[ ${#CODE_SOURCES[@]} -ne 7 ]]; then
  fail_check "expected 7 CRAWLER_SOURCE_DEFS entries, got ${#CODE_SOURCES[@]}: ${CODE_SOURCES[*]}"
else
  pass_check "CRAWLER_SOURCE_DEFS has 7 discovery sources: ${CODE_SOURCES[*]}"
fi

# --- OPERATOR_GUIDE must mention every wired source ---
for src in "${CODE_SOURCES[@]}"; do
  if grep -q "$src" "$OPERATOR_GUIDE"; then
    pass_check "OPERATOR_GUIDE mentions $src"
  else
    fail_check "OPERATOR_GUIDE missing source $src"
  fi
done

# --- No stale "4개 발견" / "4 crawler" without historical note ---
CRAWLER_DOC_TARGETS=(
  docs/OPERATOR_GUIDE.md
  docs/MVP_DESIGN.md
  docs/SEED_DATA.md
  docs/X402_OPEN_LISTING_SPEC.md
)
stale_hits=""
for doc in "${CRAWLER_DOC_TARGETS[@]}"; do
  if [[ ! -f "$doc" ]]; then
    fail_check "expected crawler alignment doc missing: $doc"
    continue
  fi
  hit=$(grep -nE '4개 발견|4 crawler' "$doc" || true)
  if [[ -n "$hit" ]]; then
    stale_hits+="${doc}:\n${hit}\n"
  fi
done
if [[ -n "$stale_hits" ]]; then
  fail_check "stale crawler count wording found:\n$stale_hits"
else
  pass_check "no stale '4개 발견' / '4 crawler' in target docs"
fi

# --- MVP_DESIGN §3 must include vendor_orgs + bazaar ---
for src in vendor_orgs bazaar mcp-registry; do
  if grep -q "$src" "$MVP_DESIGN"; then
    pass_check "MVP_DESIGN mentions $src"
  else
    fail_check "MVP_DESIGN missing $src"
  fi
done

# --- Scheduler jobs: every CRAWLER_SOURCE_DEFS source has a cron spec ---
for src in "${CODE_SOURCES[@]}"; do
  if grep -q "source: \"$src\"" "$SCHEDULER_RS"; then
    pass_check "CRAWLER_JOB_SPECS schedules $src"
  else
    fail_check "CRAWLER_JOB_SPECS missing schedule for $src"
  fi
done

# --- sync_stars is maintenance, not in CRAWLER_SOURCE_DEFS ---
if grep -q 'source: "sync_stars"' "$SCHEDULER_RS"; then
  pass_check "sync_stars maintenance job scheduled separately"
else
  fail_check "sync_stars maintenance job missing from scheduler"
fi

if [[ $fail -ne 0 ]]; then
  log ""
  log "CRAWLER DOCS ALIGNMENT: FAIL"
  exit 1
fi

log ""
log "CRAWLER DOCS ALIGNMENT: PASS"
exit 0