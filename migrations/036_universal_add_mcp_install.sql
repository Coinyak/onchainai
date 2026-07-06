-- 036_universal_add_mcp_install.sql — card-surface universal install commands for HTTP MCP tools.

UPDATE site_settings
SET mcp_endpoint = 'npx add-mcp https://www.onchain-ai.xyz/mcp'
WHERE id = 1
  AND mcp_endpoint LIKE '%mcp-remote%';

UPDATE tools
SET install_command = 'npx add-mcp https://www.onchain-ai.xyz/mcp',
    safe_copy_command = 'npx add-mcp https://www.onchain-ai.xyz/mcp',
    updated_at = now()
WHERE slug = 'onchainai';

UPDATE tools
SET install_command = 'npx add-mcp ' || trim(mcp_endpoint),
    safe_copy_command = 'npx add-mcp ' || trim(mcp_endpoint),
    updated_at = now()
WHERE mcp_endpoint ~ '^https?://'
  AND trim(mcp_endpoint) <> ''
  AND install_risk_level IN ('low', 'medium')
  AND (
    install_command ILIKE 'npx mcp-remote%'
    OR install_command ILIKE 'claude mcp add%'
    OR install_command IS NULL
    OR trim(install_command) = ''
  );