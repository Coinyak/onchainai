export const ONCHAINAI_MCP_SERVER_NAME = "onchainai";

export const SITE_ORIGIN = "https://www.onchain-ai.xyz";

export const ONCHAINAI_MCP_HTTP_URL = `${SITE_ORIGIN}/mcp`;

export function universalMcpInstallCommand(httpUrl) {
  return `npx add-mcp ${httpUrl.trim()}`;
}

export const ONCHAINAI_MCP_UNIVERSAL_CMD = universalMcpInstallCommand(ONCHAINAI_MCP_HTTP_URL);

/** Client-tab / Claude Code CLI — not for tool cards. */
export const ONCHAINAI_CLAUDE_CODE_CMD = `claude mcp add --transport http ${ONCHAINAI_MCP_SERVER_NAME} ${ONCHAINAI_MCP_HTTP_URL}`;