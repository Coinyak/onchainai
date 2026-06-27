#!/usr/bin/env bash
# Goal verification plan — single authoritative log with git provenance (harness-round-11).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRATCH="${ONCHAINAI_SCRATCH:-/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/T/grok-goal-11e98898edeb/implementer}"
LOG="$SCRATCH/verification-run.log"
BASE="${VERIFY_BASE:-http://127.0.0.1:3000}"
export PATH="${HOME}/.cargo/bin:${PATH}"
export RUSTFLAGS="${RUSTFLAGS:--C symbol-mangling-version=v0}"
export ONCHAINAI_SCRATCH="$SCRATCH"

mkdir -p "$SCRATCH"
rm -f "$SCRATCH"/click-test*.log "$SCRATCH"/post-deploy-local*.log "$SCRATCH"/verification-run-final.log

exec > >(tee "$LOG") 2>&1
echo "=== VERIFICATION RUN $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
echo "repo: $ROOT"
echo "scratch: $SCRATCH"
echo "base: $BASE"
cd "$ROOT"

echo ""
echo "=== Git provenance ==="
git rev-parse HEAD
git diff --stat HEAD~3..HEAD || git diff --stat HEAD~1..HEAD

GOAL_BASE="$(git merge-base HEAD 543468c^ 2>/dev/null || echo 543468c^)"
{
  echo "# OnchainAI CHANGED_FILES (repo paths)"
  echo "HEAD=$(git rev-parse HEAD)"
  echo "GOAL_BASE=$GOAL_BASE"
  echo ""
  git diff --name-only "$GOAL_BASE"..HEAD
} > "$SCRATCH/CHANGED_FILES.txt"
git diff "$GOAL_BASE"..HEAD > "$SCRATCH/CHANGES_FILE.patch"
git diff --stat "$GOAL_BASE"..HEAD > "$SCRATCH/CHANGES_FILE.stat"
cp "$SCRATCH/CHANGED_FILES.txt" "$SCRATCH/CHANGED_FILES.harness.txt"

echo ""
echo "=== Step 1: cargo test --features ssr ==="
STEP1_OUT="$(mktemp)"
if ! cargo test --features ssr -- --quiet 2>&1 | tee "$STEP1_OUT"; then
  echo "STEP1 FAIL: cargo test"
  exit 1
fi
if awk '
  /warning: .*never used|warning: .*dead_code/ { warned = 1; next }
  warned && /functions\.rs/ { found = 1; exit }
  { warned = 0 }
  END { exit(found ? 0 : 1) }
' "$STEP1_OUT"; then
  echo "STEP1 FAIL: dead_code warnings in functions.rs"
  exit 1
fi
rm -f "$STEP1_OUT"
echo "STEP1: PASS"

echo ""
echo "=== Step 2: release-build + verify-bundle ==="
./scripts/release-build.sh
echo "STEP2: PASS"

echo ""
echo "=== Step 3: logo head inspection (12 chains) ==="
for f in bitcoin bob ethereum solana base arbitrum optimism polygon bsc avalanche sui zksync; do
  echo "--- $f ---"
  head -c 200 "public/chains/${f}.svg"
  echo ""
done
echo "STEP3: PASS"

echo ""
echo "=== Step 4: structural markup grep ==="
grep -r 'chain-strip\|chain-tile\|preview-desktop\|preview-mobile\|bottom-sheet\|sidebar-rail-toggle\|tool-card-inner' src/components/*.rs src/pages/*.rs | head -20
grep -E 'on:click|href=' src/components/chain_strip.rs src/components/tool_card.rs src/components/sidebar.rs | head -15
echo "STEP4: PASS"

echo ""
echo "=== Step 5a: restart dev server ==="
./scripts/restart-dev.sh
sleep 2

echo ""
echo "=== Step 5a2: poll until /tools has tool-card ==="
for i in $(seq 1 30); do
  body="$(curl -sf "$BASE/tools" 2>/dev/null || true)"
  if [[ -n "$body" ]] && echo "$body" | grep -q 'tool-card'; then
    echo "tools ready after ${i} attempt(s)"
    break
  fi
  if [[ "$i" -eq 30 ]]; then
    echo "STEP5a2 FAIL: /tools never returned tool-card"
    exit 1
  fi
  sleep 2
done
echo "STEP5a2: PASS"

echo ""
echo "=== Step 5b: smoke-test ==="
./scripts/smoke-test.sh "$BASE"
echo "STEP5b: PASS"

echo ""
echo "=== Step 5c: click-test ==="
node scripts/click-test.mjs "$BASE" 2>&1 | tee "$SCRATCH/click-test.log"
echo "STEP5c: PASS"

echo ""
echo "=== Step 5d: browser-smoke ==="
node scripts/browser-smoke.mjs "$BASE"
echo "STEP5d: PASS"

echo ""
echo "=== Step 6: catalog logo tests + rendered paths ==="
cargo test --features ssr catalog_logo -- --nocapture
body="$(curl -sS "$BASE/tools")"
echo "$body" | grep -o '/chains/[a-z0-9_-]*\.svg' | sort -u
echo "$body" | grep -qiE 'error deserializing|missing field' && {
  echo "STEP6 FAIL: deser error in HTML"
  exit 1
}
echo "STEP6: PASS"

echo ""
echo "=== VERIFICATION RUN COMPLETE $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
