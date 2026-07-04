-- 031_mcp_x402_monetization.sql — Axis B: OnchainAI MCP premium tool pricing (no custody).
--
-- Operator self-service toggles premium MCP tools (compare_tools, export_toolkit).
-- Payment is HTTP 402 + PAYMENT-REQUIRED on POST /mcp; funds go directly to pay_to.
-- Default disabled: all MCP tools remain free until explicitly enabled.

ALTER TABLE site_settings
  ADD COLUMN IF NOT EXISTS mcp_premium_enabled BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS mcp_premium_pay_to_address TEXT,
  ADD COLUMN IF NOT EXISTS mcp_premium_price TEXT,
  ADD COLUMN IF NOT EXISTS mcp_premium_network TEXT NOT NULL DEFAULT 'eip155:8453',
  ADD COLUMN IF NOT EXISTS mcp_premium_asset TEXT,
  ADD COLUMN IF NOT EXISTS mcp_premium_display_price TEXT;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'site_settings_mcp_premium_network_check'
      AND conrelid = 'site_settings'::regclass
  ) THEN
    ALTER TABLE site_settings
      ADD CONSTRAINT site_settings_mcp_premium_network_check
      CHECK (mcp_premium_network ~ '^eip155:[0-9]+$' OR mcp_premium_network ~ '^solana:.+');
  END IF;
END $$;