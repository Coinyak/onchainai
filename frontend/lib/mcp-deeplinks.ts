import {
  ONCHAINAI_CLAUDE_CODE_CMD as ONCHAINAI_CLAUDE_CODE_CMD_CORE,
  ONCHAINAI_MCP_HTTP_URL as ONCHAINAI_MCP_HTTP_URL_CORE,
  ONCHAINAI_MCP_SERVER_NAME as ONCHAINAI_MCP_SERVER_NAME_CORE,
  ONCHAINAI_MCP_UNIVERSAL_CMD as ONCHAINAI_MCP_UNIVERSAL_CMD_CORE,
  universalMcpInstallCommand as universalMcpInstallCommandCore,
} from "./mcp-deeplinks-core.mjs";

export const ONCHAINAI_MCP_SERVER_NAME = ONCHAINAI_MCP_SERVER_NAME_CORE;

export const ONCHAINAI_MCP_HTTP_URL = ONCHAINAI_MCP_HTTP_URL_CORE;

export const universalMcpInstallCommand = universalMcpInstallCommandCore;

export const ONCHAINAI_MCP_UNIVERSAL_CMD = ONCHAINAI_MCP_UNIVERSAL_CMD_CORE;

/** Production /mcp responds 405 Allow: POST — streamable HTTP transport (2026-07-03 curl). */
export const ONCHAINAI_CLAUDE_CODE_CMD = ONCHAINAI_CLAUDE_CODE_CMD_CORE;

export const ONCHAINAI_PLUGIN_MARKETPLACE_CMD =
  "/plugin marketplace add Coinyak/onchainai";

export const ONCHAINAI_PLUGIN_INSTALL_CMD = "/plugin install onchainai@onchainai";

/** Cursor one-click install deeplink (Phase 9.2). */
export function cursorMcpDeeplink(
  serverName: string,
  serverConfig: Record<string, unknown>,
): string {
  const json = JSON.stringify(serverConfig);
  const configB64 =
    typeof Buffer !== "undefined"
      ? Buffer.from(json, "utf8").toString("base64")
      : btoa(json);
  const params = new URLSearchParams({
    name: serverName,
    config: configB64,
  });
  return `cursor://anysphere.cursor-deeplink/mcp/install?${params.toString()}`;
}

/** VS Code MCP install URL handler. */
export function vscodeMcpDeeplink(
  serverName: string,
  serverConfig: Record<string, unknown>,
): string {
  const payload = JSON.stringify({ name: serverName, ...serverConfig });
  return `vscode:mcp/install?${encodeURIComponent(payload)}`;
}

export function onchainaiCursorDeeplink(): string {
  return cursorMcpDeeplink(ONCHAINAI_MCP_SERVER_NAME, {
    url: ONCHAINAI_MCP_HTTP_URL,
  });
}

export function onchainaiVscodeDeeplink(): string {
  return vscodeMcpDeeplink(ONCHAINAI_MCP_SERVER_NAME, {
    type: "http",
    url: ONCHAINAI_MCP_HTTP_URL,
  });
}