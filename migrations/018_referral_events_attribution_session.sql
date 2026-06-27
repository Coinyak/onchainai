-- 018_referral_events_attribution_session.sql — avoid confusing local
-- attribution session metadata with undocumented x402 payment request fields.

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name = 'referral_events'
      AND column_name = 'referrer_session'
  ) AND NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name = 'referral_events'
      AND column_name = 'attribution_session'
  ) THEN
    ALTER TABLE referral_events
      RENAME COLUMN referrer_session TO attribution_session;
  END IF;
END $$;
