import type { Tool } from "@/lib/api";
import { ADD_MCP_INTENT, stripAddModeParams } from "@/lib/browser-query";


export type InstallPlatform = "claude" | "cursor" | "generic_mcp" | "cli_sdk";

export type CopyGate = "allow" | "reveal_first" | "blocked";

export interface GuideLink {
  label: string;
  url: string;
}

export interface ConnectGuideBlock {
  title?: string;
  steps: string[];
  copyText: string | null;
  copyLabel: string;
  configJson?: string | null;
  deeplinkHref?: string | null;
  deeplinkLabel?: string | null;
  showShellPrefix?: boolean;
}

export interface PublicInstallGuide {
  slug: string;
  tool_name: string;
  platform: string;
  risk_level: string;
  risk_reasons: string[];
  warning: string | null;
  blocked: boolean;
  copy_gate: CopyGate;
  command: string | null;
  config_json: string | null;
  copy_text: string | null;
  copy_label: string;
  steps: string[];
  docs_links: GuideLink[];
  x402_notice: string | null;
  referral_disclosure: string | null;
  /** Phase 9: per-client install blocks (ChatGPT / Claude / Cursor / VS Code / More). */
  connect_blocks?: ConnectGuideBlock[];
}

export const SITE_ORIGIN = "https://www.onchain-ai.xyz";

/** @deprecated Phase 9 — use CONNECT_CARD_CLIENTS from mcp-connect. */
export const CONNECT_CARD_PLATFORMS: InstallPlatform[] = [
  "claude",
  "cursor",
  "generic_mcp",
];

/** @deprecated Phase 9 — use TOOL_INSTALL_CLIENTS from mcp-connect. */
export const ALL_SELECTABLE_PLATFORMS: InstallPlatform[] = [
  "claude",
  "cursor",
  "generic_mcp",
  "cli_sdk",
];

export { ADD_MCP_INTENT };

export function platformLabel(platform: InstallPlatform): string {
  switch (platform) {
    case "claude":
      return "Claude";
    case "cursor":
      return "Cursor";
    case "generic_mcp":
      return "Generic MCP";
    case "cli_sdk":
      return "CLI/SDK";
  }
}

export function platformAsStr(platform: InstallPlatform): string {
  switch (platform) {
    case "claude":
      return "claude";
    case "cursor":
      return "cursor";
    case "generic_mcp":
      return "generic_mcp";
    case "cli_sdk":
      return "cli_sdk";
  }
}

export function copyLabelAria(copyLabel: string): string {
  switch (copyLabel) {
    case "Copy config":
      return "Copy config";
    case "Copy command":
      return "Copy command";
    case "Copy blocked":
      return "Copy blocked";
    default:
      return "Copy to clipboard";
  }
}

export function displayGuideText(guide: PublicInstallGuide): string {
  return guide.copy_text ?? guide.config_json ?? guide.command ?? "";
}

export function blocksStructuredConfig(riskLevel: string): boolean {
  return riskLevel === "high" || riskLevel === "critical";
}

export function copyGateForRisk(riskLevel: string): CopyGate {
  if (riskLevel === "critical") return "blocked";
  if (riskLevel === "high") return "reveal_first";
  return "allow";
}

export function copyAllowed(gate: CopyGate, copyRevealed: boolean): boolean {
  if (gate === "allow") return true;
  if (gate === "reveal_first") return copyRevealed;
  return false;
}

export function installWarningText(riskLevel: string): string | null {
  switch (riskLevel) {
    case "critical":
      return "Install blocked pending operator review. This command contains critical safety risks.";
    case "high":
      return "High-risk install command. Review carefully before running. Structured editor config is not generated for this command.";
    case "medium":
      return "Medium-risk install command. May require secrets or elevated permissions.";
    default:
      return null;
  }
}

function genericMcpRemoteCommand(endpoint: string): string | null {
  const trimmed = endpoint.trim();
  try {
    const parsed = new URL(trimmed);
    if (!["http:", "https:"].includes(parsed.protocol) || !parsed.host) return null;
  } catch {
    return null;
  }
  if (/[;&|`$()<>\n\r'\\]/.test(trimmed)) return null;
  return `npx mcp-remote '${trimmed}'`;
}

export function primaryInstallCommand(tool: Tool): string | null {
  const safe = tool.safe_copy_command?.trim();
  if (safe) return safe;
  const install = tool.install_command?.trim();
  if (install) return install;
  if (tool.type === "skill") return null;
  if (tool.mcp_endpoint) {
    return genericMcpRemoteCommand(tool.mcp_endpoint);
  }
  return null;
}

export function toolHasInstallPath(tool: Tool): boolean {
  return primaryInstallCommand(tool) !== null;
}

export function addMcpActionLabel(tool: Tool): string | null {
  if (!toolHasInstallPath(tool)) return null;
  if (tool.type === "mcp" || tool.mcp_endpoint) return "Add MCP";
  return "Use with agent";
}

export function addMcpHref(queryBase: string, slug: string): string {
  const base = stripAddModeParams(queryBase);
  const separator = base.includes("?") ? "&" : "?";
  return `${base}${separator}selected=${encodeURIComponent(slug)}&intent=${ADD_MCP_INTENT}`;
}

export function addMcpHrefFromCompare(compareSlugs: string[], toolSlug: string): string {
  const base =
    compareSlugs.length === 0
      ? "/tools"
      : `/tools?compare_tools=${encodeURIComponent(compareSlugs.join(","))}`;
  return addMcpHref(base, toolSlug);
}

function npmPackageUrl(packageName?: string | null): string | null {
  const pkg = packageName?.trim();
  if (!pkg || pkg.startsWith("http://") || pkg.startsWith("https://")) return null;
  return `https://www.npmjs.com/package/${pkg}`;
}

export function toolGuideMeta(
  tool: Tool,
): Pick<PublicInstallGuide, "docs_links" | "x402_notice" | "referral_disclosure"> {
  return {
    docs_links: docsLinksForTool(tool),
    x402_notice: x402NoticeForTool(tool),
    referral_disclosure: referralDisclosureForTool(tool),
  };
}

function docsLinksForTool(tool: Tool): GuideLink[] {
  const links: GuideLink[] = [];
  if (tool.repo_url?.trim()) {
    links.push({ label: "Repository", url: tool.repo_url.trim() });
  }
  if (tool.homepage?.trim()) {
    links.push({ label: "Homepage", url: tool.homepage.trim() });
  }
  const npmUrl = npmPackageUrl(tool.npm_package);
  if (npmUrl) links.push({ label: "npm package", url: npmUrl });
  if (
    tool.mcp_endpoint &&
    (tool.mcp_endpoint.startsWith("http://") || tool.mcp_endpoint.startsWith("https://"))
  ) {
    links.push({ label: "MCP endpoint", url: tool.mcp_endpoint });
  }
  return links;
}

function x402NoticeForTool(tool: Tool): string | null {
  if (tool.pricing !== "x402" && !tool.x402_price && !tool.referral_enabled) return null;
  const price = tool.x402_price?.trim() || "the provider's x402 price";
  return `Calls may request x402 payment (${price}). OnchainAI discloses payment metadata only and does not connect wallets or process payments.`;
}

function referralDisclosureForTool(tool: Tool): string | null {
  if (!tool.referral_enabled) return null;
  const bps = tool.referral_bps != null ? `${tool.referral_bps} bps` : "an operator-configured share";
  const model = tool.referral_model?.trim() || "attribution";
  return `OnchainAI may receive ${bps} through ${model} referral attribution.`;
}

export function claudeMcpConfig(
  serverName: string,
  install: string,
  riskLevel: string,
): string | null {
  if (blocksStructuredConfig(riskLevel) || !install.trim()) return null;

  const parts = install.trim().split(/\s+/);
  if (parts.length === 0) return null;

  const runners = new Set([
    "npx",
    "npm",
    "pnpm",
    "yarn",
    "cargo",
    "pip",
    "pip3",
    "node",
  ]);
  const runner = parts[0];
  if (!runners.has(runner)) return null;

  const args = parts.slice(1);
  const argsJson = args.map((a) => `"${a}"`).join(",");
  return `{"mcpServers":{"${serverName}":{"command":"${runner}","args":[${argsJson}]}}}`;
}

import {
  cursorMcpDeeplink,
  vscodeMcpDeeplink,
} from "@/lib/mcp-deeplinks";
export {
  TOOL_INSTALL_CLIENTS,
  toolInstallClientLabel,
  type ToolInstallClient,
} from "@/lib/mcp-connect-clients";

import type { ToolInstallClient } from "@/lib/mcp-connect-clients";

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

function toolHttpEndpoint(tool: Tool): string | null {
  const endpoint = tool.mcp_endpoint?.trim();
  if (!endpoint?.startsWith("http://") && !endpoint?.startsWith("https://")) {
    return null;
  }
  return endpoint;
}

function isMcpCatalogTool(tool: Tool): boolean {
  return tool.type === "mcp" || tool.type === "x402" || Boolean(tool.mcp_endpoint);
}

function toolStdioConfig(tool: Tool, slug: string, riskLevel: string): string | null {
  if (!isMcpCatalogTool(tool)) return null;
  const command = primaryInstallCommand(tool);
  if (!command || blocksStructuredConfig(riskLevel)) return null;
  const parts = command.trim().split(/\s+/);
  if (parts.length === 0) return null;
  return stdioMcpJsonConfig(slug, parts[0], parts.slice(1));
}

function buildToolClientBlocks(
  tool: Tool,
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

export function buildPublicInstallGuide(
  tool: Tool,
  slug: string,
  client: ToolInstallClient,
): PublicInstallGuide {
  if (tool.install_risk_level === "critical") {
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: "Install guidance blocked: critical risk pending operator review.",
      blocked: true,
      copy_gate: "blocked",
      command: null,
      config_json: null,
      copy_text: null,
      copy_label: "Copy blocked",
      steps: [
        "This tool has a critical-risk install command.",
        "Public install guidance is withheld until an operator reviews the listing.",
      ],
      connect_blocks: [],
      ...toolGuideMeta(tool),
      docs_links: [],
      x402_notice: null,
      referral_disclosure: null,
    };
  }

  const command = primaryInstallCommand(tool);
  if (tool.type === "skill" && !isMcpCatalogTool(tool)) {
    const steps = command
      ? [
          "Install the skill using the command below (e.g. clawhub or your agent skills runtime).",
          "Do not paste this into MCP server settings — skills are not MCP configs.",
          "Open the docs link for usage after install.",
        ]
      : [
          "No install command is listed for this tool.",
          "Use the repository or docs links below for setup.",
        ];
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: installWarningText(tool.install_risk_level),
      blocked: false,
      copy_gate: copyGateForRisk(tool.install_risk_level),
      command,
      config_json: null,
      copy_text: command,
      copy_label: "Copy command",
      steps,
      connect_blocks: command
        ? [
            {
              steps: ["Run the install command, then open the docs for usage."],
              copyText: command,
              copyLabel: "Copy command",
              showShellPrefix: true,
            },
          ]
        : [],
      ...toolGuideMeta(tool),
    };
  }

  if (
    (tool.type === "cli" || tool.type === "sdk" || tool.type === "api") &&
    !isMcpCatalogTool(tool)
  ) {
    const steps = command
      ? [
          "Run the install command in your terminal or package manager.",
          "Open the repository or docs link for setup and API keys.",
        ]
      : [
          "No install command is listed for this tool.",
          "Use the repository or docs links below for setup.",
        ];
    return {
      slug,
      tool_name: tool.name,
      platform: client,
      risk_level: tool.install_risk_level,
      risk_reasons: tool.install_risk_reasons,
      warning: installWarningText(tool.install_risk_level),
      blocked: false,
      copy_gate: copyGateForRisk(tool.install_risk_level),
      command,
      config_json: null,
      copy_text: command,
      copy_label: "Copy command",
      steps,
      connect_blocks: command
        ? [
            {
              steps: ["Copy and run the install command below."],
              copyText: command,
              copyLabel: "Copy command",
              showShellPrefix: true,
            },
          ]
        : [],
      ...toolGuideMeta(tool),
    };
  }

  const blocks = buildToolClientBlocks(tool, slug, client);
  const primary = blocks.find((b) => b.copyText) ?? blocks[0];
  const copyGate = copyGateForRisk(tool.install_risk_level);

  return {
    slug,
    tool_name: tool.name,
    platform: client,
    risk_level: tool.install_risk_level,
    risk_reasons: tool.install_risk_reasons,
    warning: installWarningText(tool.install_risk_level),
    blocked: false,
    copy_gate: copyGate,
    command: primaryInstallCommand(tool),
    config_json: primary.configJson ?? null,
    copy_text: primary.copyText,
    copy_label: primary.copyLabel,
    steps: primary.steps,
    connect_blocks: blocks,
    ...toolGuideMeta(tool),
  };
}