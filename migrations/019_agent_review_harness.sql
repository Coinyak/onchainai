-- 019_agent_review_harness.sql — official links, review runs, entries, operator verdicts.

CREATE TABLE IF NOT EXISTS tool_official_links (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  link_type TEXT NOT NULL,
  url TEXT NOT NULL,
  display_label TEXT NOT NULL,
  verification_status TEXT NOT NULL DEFAULT 'candidate',
  official_badge_allowed BOOLEAN NOT NULL DEFAULT false,
  evidence_strength TEXT NOT NULL DEFAULT 'weak',
  verification_method TEXT,
  discovered_from TEXT,
  verified_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  verified_at TIMESTAMPTZ,
  notes TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_official_links_link_type_check'
      AND conrelid = 'tool_official_links'::regclass
  ) THEN
    ALTER TABLE tool_official_links
      ADD CONSTRAINT tool_official_links_link_type_check
      CHECK (link_type IN ('github', 'website', 'x'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_official_links_verification_status_check'
      AND conrelid = 'tool_official_links'::regclass
  ) THEN
    ALTER TABLE tool_official_links
      ADD CONSTRAINT tool_official_links_verification_status_check
      CHECK (verification_status IN ('candidate', 'claimed', 'verified', 'rejected'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_official_links_evidence_strength_check'
      AND conrelid = 'tool_official_links'::regclass
  ) THEN
    ALTER TABLE tool_official_links
      ADD CONSTRAINT tool_official_links_evidence_strength_check
      CHECK (evidence_strength IN ('weak', 'medium', 'strong'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tool_official_links_tool_id ON tool_official_links(tool_id);
CREATE INDEX IF NOT EXISTS idx_tool_official_links_status ON tool_official_links(verification_status);

CREATE TABLE IF NOT EXISTS review_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  queue TEXT,
  runner_name TEXT NOT NULL,
  prompt_version TEXT,
  snapshot_version TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'running',
  summary TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  created_by UUID REFERENCES profiles(id) ON DELETE SET NULL
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'review_runs_status_check'
      AND conrelid = 'review_runs'::regclass
  ) THEN
    ALTER TABLE review_runs
      ADD CONSTRAINT review_runs_status_check
      CHECK (status IN ('running', 'completed', 'failed', 'discarded'));
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS review_entries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  review_run_id UUID NOT NULL REFERENCES review_runs(id) ON DELETE CASCADE,
  entry_type TEXT NOT NULL,
  role TEXT NOT NULL,
  agent_label TEXT,
  recommended_action TEXT,
  confidence REAL,
  rationale TEXT,
  supporting_evidence_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  dissent_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  missing_proofs_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'review_entries_entry_type_check'
      AND conrelid = 'review_entries'::regclass
  ) THEN
    ALTER TABLE review_entries
      ADD CONSTRAINT review_entries_entry_type_check
      CHECK (entry_type IN ('agent_review', 'operator_note', 'system_event'));
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS operator_verdicts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  review_run_id UUID REFERENCES review_runs(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  from_status TEXT,
  to_status TEXT,
  from_claim_state TEXT,
  to_claim_state TEXT,
  reason_codes TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
  note TEXT,
  operator_id UUID NOT NULL REFERENCES profiles(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_review_runs_tool_id ON review_runs(tool_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_review_entries_run_id ON review_entries(review_run_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_operator_verdicts_tool_id ON operator_verdicts(tool_id, created_at DESC);

ALTER TABLE tool_official_links ENABLE ROW LEVEL SECURITY;
ALTER TABLE review_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE review_entries ENABLE ROW LEVEL SECURITY;
ALTER TABLE operator_verdicts ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read tool official links" ON tool_official_links;
CREATE POLICY "Admin read tool official links" ON tool_official_links
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write tool official links" ON tool_official_links;
CREATE POLICY "Admin write tool official links" ON tool_official_links
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Public read verified official links" ON tool_official_links;
CREATE POLICY "Public read verified official links" ON tool_official_links
  FOR SELECT TO anon, authenticated
  USING (
    verification_status IN ('candidate', 'claimed', 'verified')
    AND tool_id IN (
      SELECT id FROM tools
      WHERE approval_status = 'approved'
        AND relevance_status = 'accepted'
        AND install_risk_level <> 'critical'
        AND quarantined_at IS NULL
    )
  );

DROP POLICY IF EXISTS "Admin read review runs" ON review_runs;
CREATE POLICY "Admin read review runs" ON review_runs
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write review runs" ON review_runs;
CREATE POLICY "Admin write review runs" ON review_runs
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin read review entries" ON review_entries;
CREATE POLICY "Admin read review entries" ON review_entries
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write review entries" ON review_entries;
CREATE POLICY "Admin write review entries" ON review_entries
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin read operator verdicts" ON operator_verdicts;
CREATE POLICY "Admin read operator verdicts" ON operator_verdicts
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write operator verdicts" ON operator_verdicts;
CREATE POLICY "Admin write operator verdicts" ON operator_verdicts
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );