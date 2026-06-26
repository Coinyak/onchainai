-- 007_submission_reports_claims.sql — user submissions, listing reports, claim flow.
--
-- Submissions enter a review queue (not public tools). Reports flag published
-- listings for operator review. Claim state tracks project ownership requests.

-- ---------------------------------------------------------------------------
-- tools: claim state
-- ---------------------------------------------------------------------------
ALTER TABLE tools
  ADD COLUMN IF NOT EXISTS claim_state TEXT NOT NULL DEFAULT 'unclaimed';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_claim_state_check' AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_claim_state_check
      CHECK (claim_state IN ('unclaimed', 'claim_pending', 'claimed', 'disputed', 'revoked'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tools_claim_state ON tools(claim_state);

-- ---------------------------------------------------------------------------
-- tool_submissions — intake queue (never directly public)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tool_submissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  submitted_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  payload JSONB NOT NULL,
  crypto_relevance_score INT NOT NULL DEFAULT 0,
  relevance_status TEXT NOT NULL DEFAULT 'needs_review',
  install_risk_level TEXT NOT NULL DEFAULT 'medium',
  rejection_reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_submissions_status_check' AND conrelid = 'tool_submissions'::regclass
  ) THEN
    ALTER TABLE tool_submissions
      ADD CONSTRAINT tool_submissions_status_check
      CHECK (status IN ('pending', 'approved', 'rejected', 'needs_info'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_submissions_relevance_status_check'
      AND conrelid = 'tool_submissions'::regclass
  ) THEN
    ALTER TABLE tool_submissions
      ADD CONSTRAINT tool_submissions_relevance_status_check
      CHECK (relevance_status IN ('accepted', 'needs_review', 'rejected'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_submissions_install_risk_level_check'
      AND conrelid = 'tool_submissions'::regclass
  ) THEN
    ALTER TABLE tool_submissions
      ADD CONSTRAINT tool_submissions_install_risk_level_check
      CHECK (install_risk_level IN ('low', 'medium', 'high', 'critical'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tool_submissions_submitted_by ON tool_submissions(submitted_by);
CREATE INDEX IF NOT EXISTS idx_tool_submissions_status ON tool_submissions(status);
CREATE INDEX IF NOT EXISTS idx_tool_submissions_created_at ON tool_submissions(created_at DESC);

ALTER TABLE tool_submissions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Self read submissions" ON tool_submissions;
CREATE POLICY "Self read submissions" ON tool_submissions
  FOR SELECT TO authenticated
  USING (submitted_by = (select auth.uid()));

DROP POLICY IF EXISTS "Self insert submissions" ON tool_submissions;
CREATE POLICY "Self insert submissions" ON tool_submissions
  FOR INSERT TO authenticated
  WITH CHECK (submitted_by = (select auth.uid()));

DROP POLICY IF EXISTS "Admin read submissions" ON tool_submissions;
CREATE POLICY "Admin read submissions" ON tool_submissions
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

-- ---------------------------------------------------------------------------
-- tool_reports — user-reported listing issues
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tool_reports (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  reported_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  reason TEXT NOT NULL,
  details TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_reports_reason_check' AND conrelid = 'tool_reports'::regclass
  ) THEN
    ALTER TABLE tool_reports
      ADD CONSTRAINT tool_reports_reason_check
      CHECK (reason IN (
        'scam_phishing',
        'unsafe_install',
        'wrong_category',
        'not_crypto_related',
        'broken_link',
        'duplicate_listing'
      ));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_reports_status_check' AND conrelid = 'tool_reports'::regclass
  ) THEN
    ALTER TABLE tool_reports
      ADD CONSTRAINT tool_reports_status_check
      CHECK (status IN ('open', 'resolved', 'dismissed'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tool_reports_tool_id ON tool_reports(tool_id);
CREATE INDEX IF NOT EXISTS idx_tool_reports_status ON tool_reports(status);
CREATE INDEX IF NOT EXISTS idx_tool_reports_created_at ON tool_reports(created_at DESC);

ALTER TABLE tool_reports ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Self read reports" ON tool_reports;
CREATE POLICY "Self read reports" ON tool_reports
  FOR SELECT TO authenticated
  USING (reported_by = (select auth.uid()));

DROP POLICY IF EXISTS "Auth insert reports" ON tool_reports;
CREATE POLICY "Auth insert reports" ON tool_reports
  FOR INSERT TO authenticated
  WITH CHECK (reported_by = (select auth.uid()));

DROP POLICY IF EXISTS "Admin read reports" ON tool_reports;
CREATE POLICY "Admin read reports" ON tool_reports
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

-- ---------------------------------------------------------------------------
-- tool_claim_requests — project claim verification (skeleton)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tool_claim_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  requested_by UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
  verification_note TEXT NOT NULL,
  contact_email TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tool_claim_requests_status_check'
      AND conrelid = 'tool_claim_requests'::regclass
  ) THEN
    ALTER TABLE tool_claim_requests
      ADD CONSTRAINT tool_claim_requests_status_check
      CHECK (status IN ('pending', 'approved', 'rejected', 'disputed', 'revoked'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tool_claim_requests_tool_id ON tool_claim_requests(tool_id);
CREATE INDEX IF NOT EXISTS idx_tool_claim_requests_requested_by ON tool_claim_requests(requested_by);

ALTER TABLE tool_claim_requests ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Self read claim requests" ON tool_claim_requests;
CREATE POLICY "Self read claim requests" ON tool_claim_requests
  FOR SELECT TO authenticated
  USING (requested_by = (select auth.uid()));

DROP POLICY IF EXISTS "Self insert claim requests" ON tool_claim_requests;
CREATE POLICY "Self insert claim requests" ON tool_claim_requests
  FOR INSERT TO authenticated
  WITH CHECK (requested_by = (select auth.uid()));

DROP POLICY IF EXISTS "Admin read claim requests" ON tool_claim_requests;
CREATE POLICY "Admin read claim requests" ON tool_claim_requests
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );