-- 017_x402_referral.sql — x402 referral attribution without custody.
--
-- OnchainAI only publishes referral/attribution metadata and records local
-- attribution events. It never proxies x402 payments or holds user/provider
-- funds. Payment verification fields below are operator trust signals, not a
-- public-visibility hard gate: unverified x402 tools may remain visible when
-- they pass the normal public quality gate.

ALTER TABLE tools
  ADD COLUMN IF NOT EXISTS referral_enabled BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS referral_bps INTEGER,
  ADD COLUMN IF NOT EXISTS referral_payout_address TEXT,
  ADD COLUMN IF NOT EXISTS referral_model TEXT,
  ADD COLUMN IF NOT EXISTS x402_pay_to_address TEXT,
  ADD COLUMN IF NOT EXISTS x402_builder_code TEXT,
  ADD COLUMN IF NOT EXISTS payment_verified BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS x402_endpoint_verified BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS price_verified BOOLEAN NOT NULL DEFAULT false;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_referral_bps_check'
      AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_referral_bps_check
      CHECK (referral_bps IS NULL OR (referral_bps >= 0 AND referral_bps <= 10000));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_referral_model_check'
      AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_referral_model_check
      CHECK (referral_model IS NULL OR referral_model IN ('split', 'attribution'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tools_referral_enabled ON tools(referral_enabled);
CREATE INDEX IF NOT EXISTS idx_tools_x402_payment_verification
  ON tools(payment_verified, x402_endpoint_verified, price_verified)
  WHERE pricing = 'x402' OR referral_enabled = true;

ALTER TABLE site_settings
  ADD COLUMN IF NOT EXISTS default_referral_bps INTEGER,
  ADD COLUMN IF NOT EXISTS default_referral_payout_address TEXT,
  ADD COLUMN IF NOT EXISTS x402_builder_code TEXT;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'site_settings_default_referral_bps_check'
      AND conrelid = 'site_settings'::regclass
  ) THEN
    ALTER TABLE site_settings
      ADD CONSTRAINT site_settings_default_referral_bps_check
      CHECK (
        default_referral_bps IS NULL
        OR (default_referral_bps >= 0 AND default_referral_bps <= 10000)
      );
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS referral_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  referrer_session TEXT,
  amount NUMERIC(20, 8),
  currency TEXT,
  tx_hash TEXT,
  chain TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'referral_events_event_type_check'
      AND conrelid = 'referral_events'::regclass
  ) THEN
    ALTER TABLE referral_events
      ADD CONSTRAINT referral_events_event_type_check
      CHECK (event_type IN ('view', 'install_guide', 'click_out', 'reported_settlement'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_referral_events_tool_created
  ON referral_events(tool_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_referral_events_event_type
  ON referral_events(event_type, created_at DESC);

ALTER TABLE referral_events ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read referral events" ON referral_events;
CREATE POLICY "Admin read referral events" ON referral_events
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

CREATE TABLE IF NOT EXISTS referral_payouts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  period TEXT NOT NULL,
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  total_amount NUMERIC(20, 8) NOT NULL DEFAULT 0,
  currency TEXT NOT NULL DEFAULT 'USDC',
  tx_hash TEXT,
  verified_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_referral_payouts_tool_period
  ON referral_payouts(tool_id, period);

ALTER TABLE referral_payouts ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read referral payouts" ON referral_payouts;
CREATE POLICY "Admin read referral payouts" ON referral_payouts
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

-- Keep RLS aligned with server PUBLIC_TOOL_WHERE. Do not add payment_verified,
-- x402_endpoint_verified, or price_verified here; those are displayed trust
-- signals rather than visibility requirements.
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
