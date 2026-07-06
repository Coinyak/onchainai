export const ONCHAINAI_MCP_SERVER_NAME = "onchainai";

export const SITE_ORIGIN = "https://www.onchain-ai.xyz";

export const ONCHAINAI_MCP_HTTP_URL = `${SITE_ORIGIN}/mcp`;

const SHELL_METACHAR_RE = /[;&|`$()<>\n\r'"\\ \t]/;

/**
 * @param {string} url
 */
export function isValidHttpMcpUrl(url) {
  const trimmed = url.trim();
  if (!trimmed || SHELL_METACHAR_RE.test(trimmed)) return false;
  try {
    const parsed = new URL(trimmed);
    return ["http:", "https:"].includes(parsed.protocol) && Boolean(parsed.host);
  } catch {
    return false;
  }
}

/**
 * @param {string} httpUrl
 * @returns {string | null}
 */
export function universalMcpInstallCommand(httpUrl) {
  const trimmed = httpUrl.trim();
  if (!isValidHttpMcpUrl(trimmed)) return null;
  return `npx add-mcp ${new URL(trimmed).href}`;
}

export const ONCHAINAI_MCP_UNIVERSAL_CMD =
  universalMcpInstallCommand(ONCHAINAI_MCP_HTTP_URL) ??
  `npx add-mcp ${ONCHAINAI_MCP_HTTP_URL}`;

/** Client-tab / Claude Code CLI — not for tool cards. */
export const ONCHAINAI_CLAUDE_CODE_CMD = `claude mcp add --transport http ${ONCHAINAI_MCP_SERVER_NAME} ${ONCHAINAI_MCP_HTTP_URL}`;