-- 028_x402_verification.sql — probe target URL + verification run metadata.
--
-- Trust flags (payment_verified, x402_endpoint_verified, price_verified) remain
-- display signals only; they are not added to PUBLIC_TOOL_WHERE or public RLS.

ALTER TABLE tools
  ADD COLUMN IF NOT EXISTS x402_endpoint TEXT,
  ADD COLUMN IF NOT EXISTS x402_last_checked_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS x402_check_failures INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_tools_x402_endpoint_probe
  ON tools(pricing, x402_endpoint)
  WHERE pricing = 'x402' AND x402_endpoint IS NOT NULL;