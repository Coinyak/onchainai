-- 031_x402_open_listing.sql — x402 open self-serve listing (X402_OPEN_LISTING_SPEC).
--
-- Adds listing-terms consent audit, referral agreement audit trail, and probe
-- history for future premium trust data. No visibility-gate changes: probe
-- verification stays a trust signal only (017/028 principle).

-- Terms consent recorded at submission time (x402 self-serve listings).
ALTER TABLE tool_submissions
  ADD COLUMN IF NOT EXISTS terms_version TEXT,
  ADD COLUMN IF NOT EXISTS terms_accepted_at TIMESTAMPTZ;

-- Referral agreement audit trail (spec §M1). One active row per tool+user;
-- revoked_at marks superseded/withdrawn agreements.
CREATE TABLE IF NOT EXISTS listing_agreements (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  user_id UUID REFERENCES profiles(id) ON DELETE SET NULL,
  terms_version TEXT NOT NULL,
  referral_bps INTEGER,
  model TEXT,
  accepted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at TIMESTAMPTZ
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'listing_agreements_bps_check'
      AND conrelid = 'listing_agreements'::regclass
  ) THEN
    ALTER TABLE listing_agreements
      ADD CONSTRAINT listing_agreements_bps_check
      CHECK (referral_bps IS NULL OR (referral_bps >= 0 AND referral_bps <= 10000));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'listing_agreements_model_check'
      AND conrelid = 'listing_agreements'::regclass
  ) THEN
    ALTER TABLE listing_agreements
      ADD CONSTRAINT listing_agreements_model_check
      CHECK (model IS NULL OR model IN ('split', 'attribution'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_listing_agreements_tool
  ON listing_agreements(tool_id) WHERE revoked_at IS NULL;

ALTER TABLE listing_agreements ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read listing agreements" ON listing_agreements;
CREATE POLICY "Admin read listing agreements" ON listing_agreements
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

-- Probe result history (spec §L4): raw material for uptime/price-history
-- trust data. Server-role writes only; admin read. tool_id is nullable so
-- pre-listing probes (submit preview) can be recorded against the URL alone.
CREATE TABLE IF NOT EXISTS x402_probe_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID REFERENCES tools(id) ON DELETE CASCADE,
  endpoint_url TEXT NOT NULL,
  probed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  status TEXT NOT NULL,
  http_status INTEGER,
  advertised_price TEXT,
  actual_price TEXT,
  latency_ms INTEGER
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'x402_probe_history_status_check'
      AND conrelid = 'x402_probe_history'::regclass
  ) THEN
    ALTER TABLE x402_probe_history
      ADD CONSTRAINT x402_probe_history_status_check
      CHECK (status IN ('live', 'dead', 'price_mismatch', 'invalid'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_x402_probe_history_tool_time
  ON x402_probe_history(tool_id, probed_at DESC);

ALTER TABLE x402_probe_history ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read x402 probe history" ON x402_probe_history;
CREATE POLICY "Admin read x402 probe history" ON x402_probe_history
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );
