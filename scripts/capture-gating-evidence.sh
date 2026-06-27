#!/usr/bin/env bash
# Capture atomic gating evidence for agent review harness verification.
# Usage: SCRATCH=/path/to/implementer ./scripts/capture-gating-evidence.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SCRATCH="${SCRATCH:-/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/T/grok-goal-b3b6e6c4a526/implementer}"
mkdir -p "$SCRATCH"

if [[ -f "$ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi

export RUSTFLAGS="${RUSTFLAGS:--C symbol-mangling-version=v0}"

{
  echo "COMMAND: cargo fmt --check"
  cargo fmt --check
} 2>&1 | tee "$SCRATCH/fmt.log"

# Force a full clippy pass so the log includes Checking + Finished (not cache-only).
cargo clean -p onchainai >/dev/null 2>&1 || true

{
  echo "COMMAND: cargo clippy --features ssr --all-targets --tests -- -W clippy::all"
  cargo clippy --features ssr --all-targets --tests -- -W clippy::all
} 2>&1 | tee "$SCRATCH/clippy.log"

{
  echo "COMMAND: ONCHAINAI_REQUIRE_DB_TESTS=1 cargo test --features ssr"
  ONCHAINAI_REQUIRE_DB_TESTS=1 cargo test --features ssr
} 2>&1 | tee "$SCRATCH/cargo-test.log"

if ! grep -q 'Checking onchainai' "$SCRATCH/clippy.log"; then
  echo "capture-gating-evidence: clippy.log missing Checking onchainai line" >&2
  exit 1
fi

if ! grep -q 'Finished' "$SCRATCH/clippy.log"; then
  echo "capture-gating-evidence: clippy.log missing Finished line" >&2
  exit 1
fi

if grep -E '^warning:|^error:' "$SCRATCH/clippy.log"; then
  echo "capture-gating-evidence: clippy reported warnings or errors" >&2
  exit 1
fi

if ! grep -q 'review_tool_server_fn_approves_claim_pending_into_claimed' "$SCRATCH/cargo-test.log"; then
  echo "capture-gating-evidence: missing review_tool_server_fn_approves_claim_pending_into_claimed" >&2
  exit 1
fi

if ! grep -q 'review_tool_server_fn_mark_official_after_claim_and_verified_links' "$SCRATCH/cargo-test.log"; then
  echo "capture-gating-evidence: missing review_tool_server_fn_mark_official_after_claim_and_verified_links" >&2
  exit 1
fi

if grep -E 'test result: FAILED| [1-9][0-9]* failed' "$SCRATCH/cargo-test.log"; then
  echo "capture-gating-evidence: cargo test reported failures" >&2
  exit 1
fi

if grep -q '^SKIP: .*review_tool' "$SCRATCH/cargo-test.log"; then
  echo "capture-gating-evidence: review_tool server function tests skipped in required DB mode" >&2
  exit 1
fi

echo "GATING EVIDENCE PASS (logs in $SCRATCH)"
