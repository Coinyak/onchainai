#!/usr/bin/env bash
# Self-contained tests for scripts/clean-build-artifacts.sh using a temp repo.
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLEANER_SOURCE="${SOURCE_ROOT}/scripts/clean-build-artifacts.sh"

fail() {
  echo "CLEAN BUILD ARTIFACTS TEST FAIL: $*" >&2
  exit 1
}

pass() {
  echo "CLEAN BUILD ARTIFACTS TEST PASS: $*"
}

assert_exists() {
  local path="$1"
  [[ -e "$path" ]] || fail "expected path to exist: ${path}"
}

assert_missing() {
  local path="$1"
  [[ ! -e "$path" ]] || fail "expected path to be removed: ${path}"
}

assert_contains() {
  local needle="$1"
  local file="$2"
  grep -Fq "$needle" "$file" || {
    cat "$file" >&2
    fail "expected output to contain: ${needle}"
  }
}

write_artifact_group() {
  local hash="$1"
  local stamp="$2"
  local deps="target/debug/deps"

  mkdir -p "$deps"
  printf 'bin-%s\n' "$hash" >"${deps}/onchainai-${hash}"
  printf 'dep-%s\n' "$hash" >"${deps}/onchainai-${hash}.d"
  printf 'rlib-%s\n' "$hash" >"${deps}/libonchainai-${hash}.rlib"
  printf 'rmeta-%s\n' "$hash" >"${deps}/libonchainai-${hash}.rmeta"
  printf 'obj-%s\n' "$hash" >"${deps}/libonchainai-${hash}.abc123.rcgu.o"
  touch -t "$stamp" \
    "${deps}/onchainai-${hash}" \
    "${deps}/onchainai-${hash}.d" \
    "${deps}/libonchainai-${hash}.rlib" \
    "${deps}/libonchainai-${hash}.rmeta" \
    "${deps}/libonchainai-${hash}.abc123.rcgu.o"
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/repo/scripts"
cp "$CLEANER_SOURCE" "$tmpdir/repo/scripts/clean-build-artifacts.sh"
chmod +x "$tmpdir/repo/scripts/clean-build-artifacts.sh"
cd "$tmpdir/repo"
CLEANER="$tmpdir/repo/scripts/clean-build-artifacts.sh"

echo "test-clean-build-artifacts: temp repo at ${tmpdir}"

write_artifact_group aaaaaaaaaaaaaaaa 202606280101
write_artifact_group bbbbbbbbbbbbbbbb 202606290101
write_artifact_group cccccccccccccccc 202606300101
write_artifact_group dddddddddddddddd 202606300201

printf 'dependency\n' > target/debug/deps/libserde-eeeeeeeeeeeeeeee.rlib
printf 'top-level\n' > target/debug/onchainai

dry_run_out="$tmpdir/dry-run.out"
"$CLEANER" --stale-main-crate --stale-main-crate-keep 2 --dry-run >"$dry_run_out"
assert_contains "[dry-run] stale main-crate group: aaaaaaaaaaaaaaaa" "$dry_run_out"
assert_contains "[dry-run] stale main-crate group: bbbbbbbbbbbbbbbb" "$dry_run_out"
assert_exists target/debug/deps/onchainai-aaaaaaaaaaaaaaaa
assert_exists target/debug/deps/onchainai-bbbbbbbbbbbbbbbb

"$CLEANER" --stale-main-crate --stale-main-crate-keep 2
assert_missing target/debug/deps/onchainai-aaaaaaaaaaaaaaaa
assert_missing target/debug/deps/libonchainai-aaaaaaaaaaaaaaaa.rlib
assert_missing target/debug/deps/libonchainai-bbbbbbbbbbbbbbbb.abc123.rcgu.o
assert_exists target/debug/deps/onchainai-cccccccccccccccc
assert_exists target/debug/deps/libonchainai-dddddddddddddddd.rmeta
assert_exists target/debug/deps/libserde-eeeeeeeeeeeeeeee.rlib
assert_exists target/debug/onchainai
pass "stale main-crate pruning keeps newest groups and dependencies"

"$CLEANER" --stale-main-crate --stale-main-crate-keep 10
assert_exists target/debug/deps/onchainai-cccccccccccccccc
assert_exists target/debug/deps/onchainai-dddddddddddddddd
pass "stale main-crate pruning is a no-op when group count is under keep limit"

symlink_repo="$tmpdir/symlink-repo"
external_debug="$tmpdir/external-debug"
mkdir -p "$symlink_repo/scripts" "$symlink_repo/target" "$external_debug/deps"
cp "$CLEANER_SOURCE" "$symlink_repo/scripts/clean-build-artifacts.sh"
chmod +x "$symlink_repo/scripts/clean-build-artifacts.sh"
ln -s "$external_debug" "$symlink_repo/target/debug"
cd "$symlink_repo"
CLEANER="$symlink_repo/scripts/clean-build-artifacts.sh"
write_artifact_group eeeeeeeeeeeeeeee 202606280101
write_artifact_group ffffffffffffffff 202606290101

symlink_out="$tmpdir/symlink.out"
set +e
"$CLEANER" --stale-main-crate --stale-main-crate-keep 1 >"$symlink_out" 2>&1
symlink_status=$?
set -e
[[ "$symlink_status" == "1" ]] || {
  cat "$symlink_out" >&2
  fail "expected symlinked deps cleanup to exit 1, got ${symlink_status}"
}
assert_contains "ERROR: target/debug is a symlink; refusing cleanup" "$symlink_out"
assert_exists "$external_debug/deps/onchainai-eeeeeeeeeeeeeeee"
assert_exists "$external_debug/deps/onchainai-ffffffffffffffff"
pass "stale main-crate pruning refuses symlinked deps"

echo "CLEAN BUILD ARTIFACTS TESTS PASS"
