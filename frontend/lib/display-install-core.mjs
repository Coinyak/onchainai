/** @typedef {{ safe_copy_command?: string | null; install_command?: string | null; type?: string; mcp_endpoint?: string | null }} InstallSurfaceTool */

import { universalMcpInstallCommand } from "./mcp-deeplinks-core.mjs";

export { universalMcpInstallCommand };

const SHELL_METACHAR_RE = /[;&|`$()<>\n\r'\\]/;
const HTTP_URL_RE = /https?:\/\/[^\s'"]+/g;
const CLIENT_MCP_CMD_RE =
  /^(?:claude\s+mcp\s+add|codex\s+mcp\s+add|cursor\s+mcp\s+add|npx\s+mcp-remote)\b/i;

/**
 * @param {string} cmd
 */
export function isClientSpecificMcpCommand(cmd) {
  const trimmed = cmd?.trim();
  return Boolean(trimmed && CLIENT_MCP_CMD_RE.test(trimmed));
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
 * @param {string} raw
 */
function resolveHttpMcpEndpoint(tool, raw) {
  if (tool.mcp_endpoint && isValidHttpMcpUrl(tool.mcp_endpoint)) {
    return tool.mcp_endpoint.trim();
  }
  return httpUrlFromMcpInstallCommand(raw);
}

/**
 * @param {InstallSurfaceTool} tool
 */
export function displayInstallCommand(tool) {
  const safe = tool.safe_copy_command?.trim() || "";
  const install = tool.install_command?.trim() || "";

  // Operator-curated copy wins unless it is a legacy/client-specific MCP string.
  if (safe && !isClientSpecificMcpCommand(safe)) {
    return safe;
  }

  const raw = safe || install;

  if (raw && isMcpCatalogTool(tool) && isClientSpecificMcpCommand(raw)) {
    const endpointUrl = resolveHttpMcpEndpoint(tool, raw);
    if (endpointUrl) {
      return universalMcpInstallCommand(endpointUrl);
    }
  }

  if (raw) return raw;

  if (tool.type !== "skill" && tool.mcp_endpoint && isValidHttpMcpUrl(tool.mcp_endpoint)) {
    return universalMcpInstallCommand(tool.mcp_endpoint);
  }

  return "";
}