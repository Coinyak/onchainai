-- Axis B premium MCP tools (export_toolkit, recommend_verified_tool, gap_audit):
-- canonical non-OKX price is $0.01 USDC on Base (eip155:8453).

UPDATE site_settings
SET
  mcp_premium_price = '$0.01',
  mcp_premium_display_price = COALESCE(
    NULLIF(TRIM(mcp_premium_display_price), ''),
    '$0.01/call'
  ),
  mcp_premium_network = COALESCE(
    NULLIF(TRIM(mcp_premium_network), ''),
    'eip155:8453'
  )
WHERE id = 1;

-- If a payee is already configured but price was empty/other, keep $0.01 as the SKU.
-- Does not force mcp_premium_enabled — operator still toggles the rail on.
