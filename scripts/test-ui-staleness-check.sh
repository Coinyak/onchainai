#!/usr/bin/env bash
# Self-contained tests for scripts/ui-staleness-check.sh using a temp git repo.
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHECKER_SOURCE="${SOURCE_ROOT}/scripts/ui-staleness-check.sh"
INC_SOURCE="${SOURCE_ROOT}/scripts/ui-watch-paths.inc.sh"

fail() {
  echo "UI STALENESS TEST FAIL: $*" >&2
  exit 1
}

pass() {
  echo "UI STALENESS TEST PASS: $*"
}

assert_exit() {
  local expected="$1"
  shift
  set +e
  "$@"
  local status=$?
  set -e
  if [[ "$status" != "$expected" ]]; then
    fail "expected exit ${expected}, got ${status}: $*"
  fi
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

setup_repo() {
  local dir="$1"
  mkdir -p "$dir/scripts"
  cp "$CHECKER_SOURCE" "$dir/scripts/ui-staleness-check.sh"
  cp "$INC_SOURCE" "$dir/scripts/ui-watch-paths.inc.sh"
  chmod +x "$dir/scripts/ui-staleness-check.sh"
  cd "$dir"
  CHECKER="$dir/scripts/ui-staleness-check.sh"
  git init -q
  git config user.email "test@example.com"
  git config user.name "Test User"

  mkdir -p src/components src/pages target/site/pkg
  cat > src/lib.rs <<'EOF'
pub fn hello() -> &'static str {
    "hello"
}
EOF
  cat > src/components/sidebar.rs <<'EOF'
pub fn sidebar() -> &'static str {
    "sidebar"
}
EOF
  touch target/site/pkg/onchainai.wasm

  git add .
  git commit -q -m "initial"
}

echo "test-ui-staleness-check: creating temp repo at ${tmpdir}"
setup_repo "$tmpdir"

# Fresh bundle: no stale UI.
assert_exit 0 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --worktree
pass "clean worktree with fresh bundle"

# Staged UI newer than bundle should fail.
sleep 1
echo "// staged change" >> src/components/sidebar.rs
git add src/components/sidebar.rs
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged UI newer than bundle"

git reset -q HEAD src/components/sidebar.rs
git checkout -q -- src/components/sidebar.rs

# Worktree UI newer than bundle should fail.
sleep 1
echo "// worktree change" >> src/components/sidebar.rs
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --worktree
pass "worktree UI newer than bundle"

git checkout -q -- src/components/sidebar.rs
touch -r target/site/pkg/onchainai.wasm src/components/sidebar.rs

# Missing bundle with staged UI should fail.
rm -f target/site/pkg/onchainai.wasm
echo "// missing bundle staged" >> src/components/sidebar.rs
git add src/components/sidebar.rs
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "missing bundle with staged UI"

git reset -q HEAD src/components/sidebar.rs
git checkout -q -- src/components/sidebar.rs

# Missing bundle with worktree UI change should fail.
echo "// missing bundle worktree" >> src/components/sidebar.rs
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --worktree
pass "missing bundle with worktree UI change"

git checkout -q -- src/components/sidebar.rs

# Bypass should succeed even when stale.
sleep 1
echo "// bypass change" >> src/components/sidebar.rs
assert_exit 0 env ONCHAINAI_SKIP_STALENESS=1 "$CHECKER" --worktree
pass "ONCHAINAI_SKIP_STALENESS bypass"

git checkout -q -- src/components/sidebar.rs

# SSR-only staged change should pass when WASM exists.
mkdir -p src/crawler
cat > src/crawler/mod.rs <<'EOF'
pub fn crawler_only() -> &'static str {
    "crawler"
}
EOF
git add src/crawler/mod.rs
assert_exit 0 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged SSR-only file with existing WASM bundle"

# Hydrate-only path (client_storage) should fail when newer than bundle.
cat > src/client_storage.rs <<'EOF'
pub fn storage_key() -> &'static str {
    "sidebar"
}
EOF
git add src/client_storage.rs
sleep 1
touch src/client_storage.rs
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged client_storage newer than bundle"

echo "UI STALENESS TESTS PASS"