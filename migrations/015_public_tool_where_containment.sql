-- Broaden public tool exclusion from exact backfill-reason array match to containment.
--
-- Migration 011 excluded score-0 rows only when crypto_relevance_reasons exactly
-- equaled the single migration-backfill reason. Rows that accumulated additional
-- reasons still matched the old equality check only when backfill was the sole
-- entry; this migration aligns RLS with server PUBLIC_TOOL_WHERE by excluding any
-- score-0 row whose reasons array contains the migration-backfill reason.

DROP POLICY IF EXISTS "Public read published tools" ON tools;

CREATE POLICY "Public read published tools" ON tools
  FOR SELECT TO anon, authenticated
  USING (
    approval_status = 'approved'
    AND relevance_status = 'accepted'
    AND NOT (
      crypto_relevance_score = 0
      AND 'migration-backfill: crypto keyword in name or description' = ANY(crypto_relevance_reasons)
    )
    AND install_risk_level <> 'critical'
    AND quarantined_at IS NULL
  );