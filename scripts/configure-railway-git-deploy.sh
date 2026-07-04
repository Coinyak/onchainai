#!/usr/bin/env bash
# One-time (idempotent): connect Railway production API to GitHub main + verify watch paths.
#
# No extra services or environments — same single Railway service, main branch only.
# Watch patterns live in railway.json (API paths only; frontend/docs-only pushes skip build).
#
# Prerequisites:
#   railway login
#   Railway GitHub App installed on Coinyak/onchainai
#
# Usage:
#   ./scripts/configure-railway-git-deploy.sh
#   ./scripts/configure-railway-git-deploy.sh --check
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RAILWAY_PROJECT_ID="${RAILWAY_PROJECT_ID:-06077d0c-6d7b-4ac2-be34-2aac44f32583}"
RAILWAY_ENVIRONMENT="${RAILWAY_ENVIRONMENT:-production}"
RAILWAY_SERVICE="${RAILWAY_SERVICE:-onchainai}"
GITHUB_REPO="${GITHUB_REPO:-Coinyak/onchainai}"
PRODUCTION_BRANCH="${PRODUCTION_BRANCH:-main}"

CHECK_ONLY=false
if [[ "${1:-}" == "--check" ]]; then
  CHECK_ONLY=true
fi

if ! command -v railway >/dev/null 2>&1; then
  echo "Install Railway CLI: npm i -g @railway/cli" >&2
  exit 1
fi

if ! railway whoami >/dev/null 2>&1; then
  echo "Not logged in. Run: railway login" >&2
  exit 1
fi

if ! /usr/bin/grep -q '"watchPatterns"' railway.json 2>/dev/null; then
  echo "railway.json missing build.watchPatterns — add API path filters first." >&2
  exit 1
fi

echo "Railway split-deploy (no extra cost):"
echo "  project=${RAILWAY_PROJECT_ID}"
echo "  environment=${RAILWAY_ENVIRONMENT}"
echo "  service=${RAILWAY_SERVICE}"
echo "  repo=${GITHUB_REPO} branch=${PRODUCTION_BRANCH}"
echo "  watchPatterns: src/, migrations/, Dockerfile.api, Cargo.*, railway.json"
echo ""

if [[ "${CHECK_ONLY}" == true ]]; then
  railway service status \
    --project "${RAILWAY_PROJECT_ID}" \
    --environment "${RAILWAY_ENVIRONMENT}" \
    --service "${RAILWAY_SERVICE}" 2>&1 || true
  echo "Check Railway dashboard → Service → Source = GitHub ${GITHUB_REPO} @ ${PRODUCTION_BRANCH}"
  exit 0
fi

echo "Connecting GitHub source (idempotent)..."
railway service source connect \
  --repo "${GITHUB_REPO}" \
  --branch "${PRODUCTION_BRANCH}" \
  --service "${RAILWAY_SERVICE}" \
  --project "${RAILWAY_PROJECT_ID}" \
  --environment "${RAILWAY_ENVIRONMENT}"

echo ""
echo "Done. Production API deploys on push to ${PRODUCTION_BRANCH} when watchPatterns match."
echo "Manual override: ./scripts/deploy-railway.sh (main only) or --force-non-main for emergencies."
echo "Spec: docs/superpowers/specs/2026-07-05-split-deploy-automation-spec.md"