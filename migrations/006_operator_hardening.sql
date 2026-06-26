-- 006_operator_hardening.sql — public visibility invariant, review gates, audit trail.
--
-- Adds relevance/install-safety columns, tightens public RLS, and records admin
-- review events. Backfill keeps previously-approved crypto-relevant listings
-- visible; others require operator re-review.

-- ---------------------------------------------------------------------------
-- tools: relevance, install safety, quarantine, review metadata
-- ---------------------------------------------------------------------------
ALTER TABLE tools
  ALTER COLUMN approval_status SET DEFAULT 'pending',
  ADD COLUMN IF NOT EXISTS crypto_relevance_score INT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS crypto_relevance_reasons TEXT[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS relevance_status TEXT NOT NULL DEFAULT 'needs_review',
  ADD COLUMN IF NOT EXISTS install_risk_level TEXT NOT NULL DEFAULT 'medium',
  ADD COLUMN IF NOT EXISTS install_risk_reasons TEXT[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS requires_secret BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS safe_copy_command TEXT,
  ADD COLUMN IF NOT EXISTS quarantined_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS last_reviewed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS review_policy_version TEXT NOT NULL DEFAULT 'operator-hardening-v1';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_relevance_score_range' AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_relevance_score_range
      CHECK (crypto_relevance_score >= 0 AND crypto_relevance_score <= 100);
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_relevance_status_check' AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_relevance_status_check
      CHECK (relevance_status IN ('accepted', 'needs_review', 'rejected'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'tools_install_risk_level_check' AND conrelid = 'tools'::regclass
  ) THEN
    ALTER TABLE tools
      ADD CONSTRAINT tools_install_risk_level_check
      CHECK (install_risk_level IN ('low', 'medium', 'high', 'critical'));
  END IF;
END $$;

-- Conservative defaults for existing rows (no mass-accept).
UPDATE tools
SET relevance_status = 'needs_review',
    install_risk_level = CASE
      WHEN install_command IS NULL OR trim(install_command) = '' THEN 'medium'
      ELSE 'medium'
    END
WHERE relevance_status = 'needs_review';

-- Backfill: keep approved listings with crypto signals publicly visible.
-- Approved rows without signals stay needs_review until operator re-reviews.
UPDATE tools
SET relevance_status = 'accepted',
    crypto_relevance_reasons = CASE
      WHEN crypto_relevance_reasons = '{}' THEN ARRAY['migration-backfill: crypto keyword in name or description']
      ELSE crypto_relevance_reasons
    END
WHERE approval_status = 'approved'
  AND relevance_status = 'needs_review'
  AND (
    lower(coalesce(name, '') || ' ' || coalesce(description, '')) ~
    '(crypto|web3|blockchain|defi|wallet|bitcoin|ethereum|onchain|nft|token|x402|rwa|solana|chain|bridge|swap|staking|lending|dex|smart.?contract|metamask|uniswap|aave|compound|polygon|arbitrum|optimism|base|bnb|usdc|usdt|dao|governance|oracle|indexer|rpc|mainnet|testnet|erc-?20|erc-?721|layer.?2|l2|mempool|validator|yield|liquidity|amm|cex|dex|mcp)'
  );

-- ---------------------------------------------------------------------------
-- Public visibility RLS (matches PUBLIC_TOOL_WHERE in src/server/queries.rs)
-- ---------------------------------------------------------------------------
DROP POLICY IF EXISTS "Public read tools" ON tools;

CREATE POLICY "Public read published tools" ON tools
  FOR SELECT TO anon, authenticated
  USING (
    approval_status = 'approved'
    AND relevance_status = 'accepted'
    AND install_risk_level <> 'critical'
    AND quarantined_at IS NULL
  );

-- ---------------------------------------------------------------------------
-- tool_review_events — admin audit trail
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tool_review_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  admin_id UUID REFERENCES profiles(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  reason TEXT NOT NULL,
  override_reason TEXT,
  before_status TEXT,
  after_status TEXT,
  snapshot_id UUID,
  recommendation_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tool_review_events_tool_id ON tool_review_events(tool_id);
CREATE INDEX IF NOT EXISTS idx_tool_review_events_created_at ON tool_review_events(created_at DESC);

ALTER TABLE tool_review_events ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read review events" ON tool_review_events;
CREATE POLICY "Admin read review events" ON tool_review_events
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );