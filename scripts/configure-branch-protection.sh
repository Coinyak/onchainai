#!/usr/bin/env bash
# Configure GitHub branch protection to require ci-success.
#
# Usage:
#   ./scripts/configure-branch-protection.sh --check-only
#   ./scripts/configure-branch-protection.sh --apply
#   ./scripts/configure-branch-protection.sh --apply --branch main
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BRANCH="main"
MODE="check-only"

usage() {
  sed -n '2,8p' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-only)
      MODE="check-only"
      shift
      ;;
    --apply)
      MODE="apply"
      shift
      ;;
    --branch)
      BRANCH="${2:-}"
      if [[ -z "$BRANCH" ]]; then
        echo "Missing value for --branch" >&2
        exit 2
      fi
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
done

REPO=""
if command -v gh >/dev/null 2>&1; then
  if gh auth status >/dev/null 2>&1; then
    REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)"
  fi
fi

print_manual_steps() {
  cat <<EOF
Manual setup (GitHub admin):
  1. Settings → Branches → Branch protection rules → Add/edit rule for: ${BRANCH}
  2. Require status checks to pass before merging
  3. Add status check: ci-success
  4. (Recommended) Require branches to be up to date before merging

Docs: docs/BRANCH_PROTECTION.md
Workflow: .github/workflows/ci.yml (job name ci-success)
EOF
}

if [[ "$MODE" == "check-only" ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "CONFIGURE BRANCH PROTECTION CHECK ONLY: gh CLI not installed"
    print_manual_steps
    exit 0
  fi
  if ! gh auth status >/dev/null 2>&1; then
    echo "CONFIGURE BRANCH PROTECTION CHECK ONLY: gh not authenticated (run: gh auth login)"
    print_manual_steps
    exit 0
  fi
  echo "CONFIGURE BRANCH PROTECTION CHECK ONLY PASS"
  echo "  repo:   ${REPO:-unknown}"
  echo "  branch: ${BRANCH}"
  echo "  check:  ci-success"
  print_manual_steps
  exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "CONFIGURE BRANCH PROTECTION FAIL: gh CLI not installed" >&2
  print_manual_steps
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "CONFIGURE BRANCH PROTECTION FAIL: gh not authenticated (run: gh auth login)" >&2
  print_manual_steps
  exit 1
fi

if [[ -z "$REPO" ]]; then
  echo "CONFIGURE BRANCH PROTECTION FAIL: could not resolve repo from gh" >&2
  exit 1
fi

echo "Applying branch protection for ${REPO}:${BRANCH} (requires admin)..."

protection_body="$(mktemp -t onchainai-branch-protection.XXXXXX.json)"
trap 'rm -f "${protection_body}"' EXIT
cat >"${protection_body}" <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci-success"]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

api_err="$(mktemp -t onchainai-branch-protection-err.XXXXXX.log)"
if ! gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/${REPO}/branches/${BRANCH}/protection" \
  --input "${protection_body}" \
  2>"${api_err}"; then
  echo "CONFIGURE BRANCH PROTECTION APPLY FAIL: GitHub API error" >&2
  cat "${api_err}" >&2
  rm -f "${api_err}"
  print_manual_steps
  exit 1
fi
rm -f "${api_err}"

verify_err="$(mktemp -t onchainai-branch-protection-verify.XXXXXX.log)"
check_index="$(gh api \
  -H "Accept: application/vnd.github+json" \
  "/repos/${REPO}/branches/${BRANCH}/protection/required_status_checks" \
  --jq '.contexts | index("ci-success")' \
  2>"${verify_err}" || true)"
if [[ -z "$check_index" || "$check_index" == "null" ]]; then
  echo "CONFIGURE BRANCH PROTECTION APPLY FAIL: ci-success not in required status checks after apply" >&2
  cat "${verify_err}" >&2
  rm -f "${verify_err}"
  print_manual_steps
  exit 1
fi
rm -f "${verify_err}"

echo "CONFIGURE BRANCH PROTECTION APPLY PASS (${REPO}:${BRANCH}, check=ci-success, enforce_admins=true)"
exit 0