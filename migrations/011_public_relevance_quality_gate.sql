-- Tighten public relevance quality after operator hardening.
--
-- Migration 006 intentionally kept legacy approved rows visible when they had
-- broad crypto-ish keywords. In live data that let generic MCP directories
-- through with score 0 and only the synthetic migration-backfill reason. Move
-- those rows back to review and make the public RLS gate match server queries.

UPDATE tools
SET relevance_status = 'needs_review',
    crypto_relevance_reasons = ARRAY[
      'quality-gate: legacy migration backfill requires operator review'
    ]
WHERE approval_status = 'approved'
  AND relevance_status = 'accepted'
  AND crypto_relevance_score = 0
  AND crypto_relevance_reasons = ARRAY[
    'migration-backfill: crypto keyword in name or description'
  ]::TEXT[];

DROP POLICY IF EXISTS "Public read published tools" ON tools;

CREATE POLICY "Public read published tools" ON tools
  FOR SELECT TO anon, authenticated
  USING (
    approval_status = 'approved'
    AND relevance_status = 'accepted'
    AND NOT (
      crypto_relevance_score = 0
      AND crypto_relevance_reasons = ARRAY[
        'migration-backfill: crypto keyword in name or description'
      ]::TEXT[]
    )
    AND install_risk_level <> 'critical'
    AND quarantined_at IS NULL
  );
