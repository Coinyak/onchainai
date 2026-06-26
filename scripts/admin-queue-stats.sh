#!/usr/bin/env bash
# Print operator review queue counts (uses DATABASE_URL from .env).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ -f .env ]]; then set -a; source .env; set +a; fi
: "${DATABASE_URL:?}"

if ! command -v psql >/dev/null 2>&1; then
  echo "psql required" >&2
  exit 1
fi

psql "$DATABASE_URL" -c "
SELECT 'public_tools' AS queue, COUNT(*) FROM tools
  WHERE approval_status='approved' AND relevance_status='accepted'
    AND install_risk_level <> 'critical' AND quarantined_at IS NULL
UNION ALL SELECT 'new_candidate', COUNT(*) FROM tools
  WHERE approval_status='pending' AND last_reviewed_at IS NULL AND quarantined_at IS NULL
UNION ALL SELECT 'known_update', COUNT(*) FROM tools
  WHERE approval_status='approved' AND last_reviewed_at IS NOT NULL
    AND updated_at > last_reviewed_at AND quarantined_at IS NULL
UNION ALL SELECT 'needs_manual_research', COUNT(*) FROM tools
  WHERE approval_status IN ('pending','approved') AND relevance_status='needs_review'
    AND crypto_relevance_score < 50 AND quarantined_at IS NULL
UNION ALL SELECT 'low_relevance', COUNT(*) FROM tools
  WHERE approval_status='pending' AND relevance_status='rejected' AND quarantined_at IS NULL
UNION ALL SELECT 'high_risk_install', COUNT(*) FROM tools
  WHERE approval_status IN ('pending','approved') AND install_risk_level IN ('high','critical')
    AND quarantined_at IS NULL
UNION ALL SELECT 'open_reports', COUNT(*) FROM tool_reports WHERE status='open'
ORDER BY queue;
"