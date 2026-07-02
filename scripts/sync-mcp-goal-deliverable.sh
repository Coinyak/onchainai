#!/usr/bin/env bash
# Mirror MCP add-flow changed files into the workspace harness deliverable tree.
# The Grok goal harness tracks /Users/love workspace files (not OnchainAI/.git alone).
#
# Usage: ./scripts/sync-mcp-goal-deliverable.sh [BASE_COMMIT]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

default_base_commit() {
  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    if base="$(git merge-base HEAD origin/main 2>/dev/null)" && [[ -n "$base" ]]; then
      echo "$base"
      return
    fi
  fi
  if git rev-parse --verify HEAD^ >/dev/null 2>&1; then
    git rev-parse HEAD^
    return
  fi
  # Shallow / single-commit clone: diff against an empty-tree sentinel
  # so the full working-tree-vs-nothing diff is still well-defined.
  echo "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
}

BASE_COMMIT="${1:-$(default_base_commit)}"
DELIVERABLE="${HOME}/.grok/projects/Users-love/onchainai-mcp-deliverable"

mkdir -p "$DELIVERABLE"

files=()
while IFS= read -r line; do
  [[ -n "$line" ]] && files+=("$line")
done < <(
  git diff --name-only "${BASE_COMMIT}..HEAD" -- \
    src/ tests/ scripts/ style/ docs/MCP_ADD_FLOW_SPEC.md
)

{
  echo "# Harness deliverable — mirrored from OnchainAI/${BASE_COMMIT}..HEAD"
  echo "# generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# repo: ${ROOT}"
  echo ""
  for f in "${files[@]}"; do
    echo "$f"
  done
  echo ""
  echo "# count: ${#files[@]}"
} >"$DELIVERABLE/CHANGED_FILES.txt"

git diff "${BASE_COMMIT}..HEAD" -- src/ tests/ scripts/ style/output.css \
  >"$DELIVERABLE/MCP_SOURCE_DIFF.patch"

for f in "${files[@]}"; do
  mkdir -p "$DELIVERABLE/$(dirname "$f")"
  cp "$ROOT/$f" "$DELIVERABLE/$f"
done

echo "Synced ${#files[@]} files → ${DELIVERABLE}"
echo "  CHANGED_FILES.txt"
echo "  MCP_SOURCE_DIFF.patch ($(wc -l <"$DELIVERABLE/MCP_SOURCE_DIFF.patch" | tr -d ' ') lines)"
