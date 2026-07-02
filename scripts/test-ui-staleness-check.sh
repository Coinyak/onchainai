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

  mkdir -p frontend/components frontend/.next src/crawler
  cat > frontend/components/Sidebar.tsx <<'EOF'
export function Sidebar() {
  return <nav>sidebar</nav>;
}
EOF
  echo "build-1" > frontend/.next/BUILD_ID
  cat > src/crawler/mod.rs <<'EOF'
pub fn crawler_only() -> &'static str {
    "crawler"
}
EOF

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
echo "// staged change" >> frontend/components/Sidebar.tsx
git add frontend/components/Sidebar.tsx
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged UI newer than bundle"

git reset -q HEAD frontend/components/Sidebar.tsx
git checkout -q -- frontend/components/Sidebar.tsx

# Worktree UI newer than bundle should fail.
sleep 1
echo "// worktree change" >> frontend/components/Sidebar.tsx
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --worktree
pass "worktree UI newer than bundle"

git checkout -q -- frontend/components/Sidebar.tsx
touch -r frontend/.next/BUILD_ID frontend/components/Sidebar.tsx

# Missing bundle with staged UI should fail.
rm -f frontend/.next/BUILD_ID
echo "// missing bundle staged" >> frontend/components/Sidebar.tsx
git add frontend/components/Sidebar.tsx
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "missing bundle with staged UI"

git reset -q HEAD frontend/components/Sidebar.tsx
git checkout -q -- frontend/components/Sidebar.tsx
echo "build-1" > frontend/.next/BUILD_ID

# Missing bundle with worktree UI change should fail.
rm -f frontend/.next/BUILD_ID
echo "// missing bundle worktree" >> frontend/components/Sidebar.tsx
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --worktree
pass "missing bundle with worktree UI change"

git checkout -q -- frontend/components/Sidebar.tsx
echo "build-1" > frontend/.next/BUILD_ID

# Bypass should succeed even when stale.
sleep 1
echo "// bypass change" >> frontend/components/Sidebar.tsx
assert_exit 0 env ONCHAINAI_SKIP_STALENESS=1 "$CHECKER" --worktree
pass "ONCHAINAI_SKIP_STALENESS bypass"

git checkout -q -- frontend/components/Sidebar.tsx

# Backend-only staged change should pass when frontend bundle exists.
git add src/crawler/mod.rs
assert_exit 0 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged backend-only file with existing frontend bundle"

# Frontend lib path should fail when newer than bundle.
mkdir -p frontend/lib
cat > frontend/lib/browser-query.ts <<'EOF'
export function queryKey() {
  return "sidebar";
}
EOF
git add frontend/lib/browser-query.ts
sleep 1
touch frontend/lib/browser-query.ts
assert_exit 2 env -u ONCHAINAI_SKIP_STALENESS "$CHECKER" --staged
pass "staged frontend/lib newer than bundle"

echo "UI STALENESS TESTS PASS"