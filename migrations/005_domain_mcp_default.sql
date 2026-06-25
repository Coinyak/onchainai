-- Update site_settings MCP default to canonical production domain (post-launch migration).
UPDATE site_settings
SET mcp_endpoint = 'npx mcp-remote www.onchain-ai.xyz/mcp'
WHERE id = 1 AND mcp_endpoint LIKE '%onchainai.xyz%';