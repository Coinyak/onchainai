/** @typedef {{ safe_copy_command?: string | null; install_command?: string | null; type?: string; mcp_endpoint?: string | null }} InstallSurfaceTool */

const SHELL_METACHAR_RE = /[;&|`$()<>\n\r'\\]/;
const HTTP_URL_RE = /https?:\/\/[^\s'"]+/g;
const CLIENT_MCP_CMD_RE =
  /^(?:claude\s+mcp\s+add|codex\s+mcp\s+add|cursor\s+mcp\s+add|npx\s+(?:add-mcp|mcp-remote))\b/i;

/**
 * @param {string} httpUrl
 */
export function universalMcpInstallCommand(httpUrl) {
  return `npx add-mcp ${httpUrl.trim()}`;
}

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
 * @param {string} cmd
 */
export function httpUrlFromMcpInstallCommand(cmd) {
  const trimmed = cmd.trim();
  if (!trimmed) return null;

  const matches = trimmed.match(HTTP_URL_RE);
  if (matches?.length) {
    const candidate = matches[matches.length - 1];
    if (isValidHttpMcpUrl(candidate)) return candidate;
  }

  for (const token of trimmed.split(/\s+/).reverse()) {
    if (token.startsWith("http://") || token.startsWith("https://")) {
      if (isValidHttpMcpUrl(token)) return token;
      continue;
    }
    if (
      token.includes(".") &&
      !token.startsWith("mcp-remote") &&
      token !== "npx" &&
      token !== "add-mcp"
    ) {
      const hostUrl = `https://${token}`;
      if (isValidHttpMcpUrl(hostUrl)) return hostUrl;
    }
  }

  return null;
}

/**
 * @param {InstallSurfaceTool} tool
 */
function isMcpCatalogTool(tool) {
  return tool.type === "mcp" || tool.type === "x402" || Boolean(tool.mcp_endpoint);
}

/**
 * @param {InstallSurfaceTool} tool
 */
function shouldUniversalizeMcpCommand(tool, raw) {
  if (!isMcpCatalogTool(tool)) return false;
  if (tool.mcp_endpoint && isValidHttpMcpUrl(tool.mcp_endpoint)) return true;
  if (raw && CLIENT_MCP_CMD_RE.test(raw)) return true;
  return false;
}

/**
 * @param {InstallSurfaceTool} tool
 */
export function displayInstallCommand(tool) {
  const raw = tool.safe_copy_command?.trim() || tool.install_command?.trim() || "";

  const endpointUrl =
    tool.mcp_endpoint && isValidHttpMcpUrl(tool.mcp_endpoint)
      ? tool.mcp_endpoint.trim()
      : httpUrlFromMcpInstallCommand(raw);

  if (endpointUrl && shouldUniversalizeMcpCommand(tool, raw)) {
    return universalMcpInstallCommand(endpointUrl);
  }

  if (raw) return raw;

  if (tool.type !== "skill" && tool.mcp_endpoint && isValidHttpMcpUrl(tool.mcp_endpoint)) {
    return universalMcpInstallCommand(tool.mcp_endpoint);
  }

  return "";
}