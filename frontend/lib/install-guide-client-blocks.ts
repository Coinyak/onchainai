/** Platform-specific install client blocks. */
import type { PublicTool } from "@/lib/api";
import type { ToolInstallClient } from "@/lib/mcp-connect-clients";
import {
  cursorMcpDeeplink,
  vscodeMcpDeeplink,
} from "@/lib/mcp-deeplinks";
import {
  type ConnectGuideBlock,
  SITE_ORIGIN,
  blocksStructuredConfig,
} from "./install-guide-shared";
import { claudeMcpConfig, primaryInstallCommand } from "./install-guide-commands";

function stdioMcpJsonConfig(
  serverName: string,
  command: string,
  args: string[],
): string {
  return JSON.stringify(
    { mcpServers: { [serverName]: { command, args } } },
    null,
    2,
  );
}

function toolHttpEndpoint(tool: PublicTool): string | null {
  const endpoint = tool.mcp_endpoint?.trim();
  if (!endpoint?.startsWith("http://") && !endpoint?.startsWith("https://")) {
    return null;
  }
  return endpoint;
}

export function isMcpCatalogTool(tool: PublicTool): boolean {
  return tool.type === "mcp" || tool.type === "x402" || Boolean(tool.mcp_endpoint);
}

function toolStdioConfig(tool: PublicTool, slug: string, riskLevel: string): string | null {
  if (!isMcpCatalogTool(tool)) return null;
  const command = primaryInstallCommand(tool);
  if (!command || blocksStructuredConfig(riskLevel)) return null;
  const parts = command.trim().split(/\s+/);
  if (parts.length === 0) return null;
  return stdioMcpJsonConfig(slug, parts[0], parts.slice(1));
}

export function buildToolClientBlocks(
  tool: PublicTool,
  slug: string,
  client: ToolInstallClient,
): ConnectGuideBlock[] {
  const riskLevel = tool.install_risk_level;
  const command = primaryInstallCommand(tool);
  const httpUrl = toolHttpEndpoint(tool);
  const stdioJson = toolStdioConfig(tool, slug, riskLevel);
  const claudeJson =
    !blocksStructuredConfig(riskLevel) && command
      ? claudeMcpConfig(slug, command, riskLevel)
      : null;

  switch (client) {
    case "generic": {
      if (httpUrl) {
        const httpJson = JSON.stringify(
          { mcpServers: { [slug]: { type: "http", url: httpUrl } } },
          null,
          2,
        );
        const stdioJson = stdioMcpJsonConfig(slug, "npx", ["mcp-remote", httpUrl]);
        return [
          {
            title: "HTTP config",
            steps: [
              "Paste the JSON into any MCP client that supports streamable HTTP.",
              "No API key required for public read-only tools.",
            ],
            copyText: httpJson,
            copyLabel: "Copy config",
            configJson: httpJson,
          },
          {
            title: "Stdio bridge",
            steps: ["For clients that only support stdio MCP."],
            copyText: stdioJson,
            copyLabel: "Copy config",
            configJson: stdioJson,
          },
        ];
      }
      return [
        {
          steps: [
            "Run the install command in your terminal.",
            "Use npx or your package manager as shown below.",
          ],
          copyText: command,
          copyLabel: "Copy command",
          showShellPrefix: true,
        },
      ];
    }
    case "codex": {
      const codexCopy = httpUrl
        ? `codex mcp add ${slug} --url ${httpUrl}`
        : command;
      return [
        {
          title: "Codex CLI",
          steps: [
            "Install Codex CLI: npm i -g @openai/codex",
            "Run the command below to register this tool's MCP server.",
            "Sign in to Codex if prompted — the tool endpoint itself may need no API key.",
          ],
          copyText: codexCopy,
          copyLabel: "Copy command",
          showShellPrefix: true,
        },
      ];
    }
    case "chatgpt":
      if (httpUrl) {
        return [
          {
            steps: [
              "Enable Developer mode in ChatGPT connector settings.",
              "Create a connector with this tool's MCP URL.",
              "Use Developer mode in chat to call the connector.",
            ],
            copyText: httpUrl,
            copyLabel: "Copy endpoint URL",
          },
        ];
      }
      return [
        {
          steps: [
            "ChatGPT connectors require an HTTP MCP endpoint.",
            "Use Claude, Cursor, VS Code, or More for CLI/SDK install instead.",
          ],
          copyText: command,
          copyLabel: "Copy command",
          showShellPrefix: true,
        },
      ];
    case "claude":
      if (httpUrl) {
        return [
          {
            title: "Claude Desktop or Web",
            steps: [
              "Add a custom connector with the MCP URL below.",
              "Enable the connector in your Claude session.",
            ],
            copyText: httpUrl,
            copyLabel: "Copy endpoint URL",
          },
          {
            title: "Claude Code CLI",
            steps: ["Register the remote MCP server with HTTP transport."],
            copyText: `claude mcp add --transport http ${slug} ${httpUrl}`,
            copyLabel: "Copy command",
            showShellPrefix: true,
          },
        ];
      }
      return [
        {
          title: "Claude Desktop",
          steps: [
            "Paste the structured MCP config into Claude settings.",
            "Restart Claude to load the tool.",
          ],
          copyText: claudeJson ?? command,
          copyLabel: claudeJson ? "Copy config" : "Copy command",
          configJson: claudeJson,
          showShellPrefix: !claudeJson,
        },
        {
          title: "Claude Code CLI",
          steps: ["Run the install command if structured config is unavailable."],
          copyText: command,
          copyLabel: "Copy command",
          showShellPrefix: true,
        },
      ];
    case "cursor": {
      const cursorConfig = httpUrl
        ? { url: httpUrl }
        : stdioJson
          ? (JSON.parse(stdioJson).mcpServers[slug] as Record<string, unknown>)
          : null;
      const configJson =
        httpUrl && cursorConfig
          ? JSON.stringify({ mcpServers: { [slug]: cursorConfig } }, null, 2)
          : stdioJson;
      const deeplink =
        cursorConfig && !blocksStructuredConfig(riskLevel)
          ? cursorMcpDeeplink(slug, cursorConfig)
          : null;
      return [
        {
          steps: [
            deeplink
              ? "Click Add to Cursor or paste the JSON into .cursor/mcp.json."
              : "Paste the JSON into .cursor/mcp.json.",
            "Reload MCP servers in Cursor.",
          ],
          copyText: configJson ?? command,
          copyLabel: configJson ? "Copy config" : "Copy command",
          configJson,
          deeplinkHref: deeplink,
          deeplinkLabel: deeplink ? "Add to Cursor" : undefined,
          showShellPrefix: !configJson,
        },
      ];
    }
    case "vscode": {
      const vscodeConfig = httpUrl
        ? { type: "http", url: httpUrl }
        : command && !blocksStructuredConfig(riskLevel)
          ? (() => {
              const parts = command.trim().split(/\s+/);
              return { type: "stdio", command: parts[0], args: parts.slice(1) };
            })()
          : null;
      const deeplink =
        vscodeConfig && !blocksStructuredConfig(riskLevel)
          ? vscodeMcpDeeplink(slug, vscodeConfig)
          : null;
      return [
        {
          steps: [
            deeplink
              ? "Click Add to VS Code or use MCP: Add Server manually."
              : "Use MCP: Add Server and paste the install command output.",
            "Start the server from MCP: List Servers.",
          ],
          copyText: httpUrl ?? command,
          copyLabel: httpUrl ? "Copy endpoint URL" : "Copy command",
          deeplinkHref: deeplink,
          deeplinkLabel: deeplink ? "Add to VS Code" : undefined,
          showShellPrefix: !httpUrl,
        },
      ];
    }
    case "more":
      return [
        {
          title: "Terminal install",
          steps: [
            "Run the install command in your terminal.",
            "Use npx or your package manager as shown below.",
          ],
          copyText: command,
          copyLabel: "Copy command",
          showShellPrefix: true,
        },
        {
          title: "More clients",
          steps: [
            "Windsurf, Gemini, Goose, Devin, Raycast, and Generic JSON are on the Connect page.",
          ],
          copyText: null,
          copyLabel: "Copy",
        },
      ];
  }
}

