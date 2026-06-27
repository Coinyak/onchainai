-- 021_official_links_hardening.sql — dedupe official links, enforce uniqueness, tighten public RLS.

DELETE FROM tool_official_links a
USING tool_official_links b
WHERE a.id > b.id
  AND a.tool_id = b.tool_id
  AND a.link_type = b.link_type
  AND a.url = b.url;

CREATE UNIQUE INDEX IF NOT EXISTS idx_tool_official_links_tool_type_url_unique
  ON tool_official_links (tool_id, link_type, url);

DROP POLICY IF EXISTS "Public read verified official links" ON tool_official_links;
CREATE POLICY "Public read verified official links" ON tool_official_links
  FOR SELECT TO anon, authenticated
  USING (
    verification_status IN ('claimed', 'verified')
    AND tool_id IN (
      SELECT id FROM tools
      WHERE approval_status = 'approved'
        AND relevance_status = 'accepted'
        AND install_risk_level <> 'critical'
        AND quarantined_at IS NULL
    )
  );