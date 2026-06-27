-- 020_public_official_links_policy.sql — align RLS with public-safe official link statuses.

DROP POLICY IF EXISTS "Public read verified official links" ON tool_official_links;
CREATE POLICY "Public read verified official links" ON tool_official_links
  FOR SELECT TO anon, authenticated
  USING (
    verification_status IN ('candidate', 'claimed', 'verified')
    AND verification_status <> 'rejected'
    AND tool_id IN (
      SELECT id FROM tools
      WHERE approval_status = 'approved'
        AND relevance_status = 'accepted'
        AND install_risk_level <> 'critical'
        AND quarantined_at IS NULL
    )
  );