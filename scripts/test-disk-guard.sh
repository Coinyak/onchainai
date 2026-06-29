#!/usr/bin/env bash
# Self-contained tests for scripts/disk-guard.sh using fake df/du.
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DISK_GUARD_SOURCE="${SOURCE_ROOT}/scripts/disk-guard.sh"

fail() {
  echo "DISK GUARD TEST FAIL: $*" >&2
  exit 1
}

pass() {
  echo "DISK GUARD TEST PASS: $*"
}

assert_contains() {
  local needle="$1"
  local file="$2"
  grep -Fq -- "$needle" "$file" || {
    cat "$file" >&2
    fail "expected output to contain: ${needle}"
  }
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/repo/scripts" "$tmpdir/repo/target" "$tmpdir/bin"
cp "$DISK_GUARD_SOURCE" "$tmpdir/repo/scripts/disk-guard.sh"
chmod +x "$tmpdir/repo/scripts/disk-guard.sh"

cat > "$tmpdir/repo/scripts/clean-build-artifacts.sh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${FAKE_CLEAN_LOG:?missing FAKE_CLEAN_LOG}"
EOF
chmod +x "$tmpdir/repo/scripts/clean-build-artifacts.sh"

cat > "$tmpdir/bin/df" <<'EOF'
#!/usr/bin/env bash
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n'
printf '/dev/fake 999999999 0 104857600 0%% /\n'
EOF
chmod +x "$tmpdir/bin/df"

cat > "$tmpdir/bin/du" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == "-sk" && "$2" == "target" ]]; then
  if grep -Fq -- '--stale-main-crate' "${FAKE_CLEAN_LOG:?missing FAKE_CLEAN_LOG}" 2>/dev/null; then
    printf '10485760\ttarget\n'
  else
    printf '17825792\ttarget\n'
  fi
  exit 0
fi
printf '17G\ttarget\n'
EOF
chmod +x "$tmpdir/bin/du"

cd "$tmpdir/repo"
log="$tmpdir/clean.log"
: > "$log"

PATH="$tmpdir/bin:$PATH" \
FAKE_CLEAN_LOG="$log" \
ONCHAINAI_STALE_MAIN_CRATE_PRUNE_GB=16 \
ONCHAINAI_STALE_MAIN_CRATE_KEEP=3 \
  ./scripts/disk-guard.sh >"$tmpdir/disk-guard.out"

assert_contains "--snapshots-only" "$log"
assert_contains "--stale-main-crate --stale-main-crate-keep 3" "$log"
pass "stale main-crate prune runs when target exceeds prune threshold"

echo "DISK GUARD TESTS PASS"
