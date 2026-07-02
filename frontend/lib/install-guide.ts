import type { Tool } from "@/lib/api";
import { ADD_MCP_INTENT, stripAddModeParams } from "@/lib/browser-query";

export type InstallPlatform = "claude" | "cursor" | "generic_mcp" | "cli_sdk";

export type CopyGate = "allow" | "reveal_first" | "blocked";

export interface GuideLink {
  label: string;
  url: string;
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
}

export const SITE_ORIGIN = "https://www.onchain-ai.xyz";

export const CONNECT_CARD_PLATFORMS: InstallPlatform[] = [
  "claude",
  "cursor",
  "generic_mcp",
];

export const ALL_SELECTABLE_PLATFORMS: InstallPlatform[] = [
  "claude",
  "cursor",
  "generic_mcp",
  "cli_sdk",
];

export const DEFAULT_CONNECT_PLATFORM: InstallPlatform = "claude";

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

function blockedGuide(tool: Tool, slug: string, platform: InstallPlatform): PublicInstallGuide {
  return {
    slug,
    tool_name: tool.name,
    platform: platformAsStr(platform),
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
      "Contact the project directly or wait for operator approval.",
    ],
    docs_links: docsLinksForTool(tool),
    x402_notice: x402NoticeForTool(tool),
    referral_disclosure: referralDisclosureForTool(tool),
  };
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

export function buildOnchainaiConnectGuide(
  platform: InstallPlatform,
  endpointCmd: string,
): PublicInstallGuide {
  const slug = "onchainai";
  const riskLevel = "low";

  let configJson: string | null = null;
  let copyText: string | null = null;
  let copyLabel: string;
  let steps: string[];

  if (platform === "claude" || platform === "cursor") {
    const config = claudeMcpConfig(slug, endpointCmd, riskLevel);
    configJson = config;
    copyText = config ?? endpointCmd;
    copyLabel = "Copy config";
    steps = [
      "Open your MCP client settings.",
      "Paste the OnchainAI search MCP config.",
      "Reload or restart your client.",
    ];
  } else {
    copyText = endpointCmd;
    copyLabel = "Copy command";
    steps = [
      "Run the command in your terminal to connect OnchainAI search MCP.",
    ];
  }

  return {
    slug,
    tool_name: "OnchainAI MCP",
    platform: platformAsStr(platform),
    risk_level: riskLevel,
    risk_reasons: ["documented package manager install"],
    warning: null,
    blocked: false,
    copy_gate: "allow",
    command: endpointCmd,
    config_json: configJson,
    copy_text: copyText,
    copy_label: copyLabel,
    steps,
    docs_links: [{ label: "OnchainAI", url: SITE_ORIGIN }],
    x402_notice: null,
    referral_disclosure: null,
  };
}

export function buildPublicInstallGuide(
  tool: Tool,
  slug: string,
  platform: InstallPlatform,
): PublicInstallGuide {
  if (tool.install_risk_level === "critical") {
    return blockedGuide(tool, slug, platform);
  }

  const riskLevel = tool.install_risk_level;
  const copyGate = copyGateForRisk(riskLevel);
  const configBlocked = blocksStructuredConfig(riskLevel);
  const command = primaryInstallCommand(tool);
  const rawInstall = tool.install_command?.trim() || null;
  const installForConfig = command ?? rawInstall ?? "";

  let configJson: string | null = null;
  let copyText: string | null = null;
  let copyLabel = "Copy command";
  let steps: string[] = [];

  switch (platform) {
    case "claude": {
      const config = !configBlocked
        ? claudeMcpConfig(slug, installForConfig, riskLevel)
        : null;
      configJson = config;
      copyText = config ?? command;
      copyLabel = configBlocked ? "Copy command" : "Copy config";
      steps = [
        "Open Claude Desktop settings.",
        configBlocked
          ? "Structured config is unavailable for high-risk commands; use generic install only if you trust the source."
          : "Paste the structured MCP config JSON into your Claude settings.",
        "Restart Claude to load the tool.",
      ];
      break;
    }
    case "cursor": {
      const config = !configBlocked
        ? claudeMcpConfig(slug, installForConfig, riskLevel)
        : null;
      configJson = config;
      copyText = config ?? command;
      copyLabel = configBlocked ? "Copy command" : "Copy config";
      steps = [
        "Open Cursor MCP settings.",
        configBlocked
          ? "High-risk install: do not paste raw shell wrappers. Add manually only if you trust the source."
          : "Paste the config JSON or use the install command.",
        "Reload MCP servers.",
      ];
      break;
    }
    case "generic_mcp":
      copyText = command;
      copyLabel = "Copy command";
      steps = ["Run the install command in your terminal."];
      break;
    case "cli_sdk":
      copyText = command;
      copyLabel = "Copy command";
      steps = [
        "Install the package using the command below.",
        "Open the docs or repository link for setup details.",
      ];
      break;
  }

  return {
    slug,
    tool_name: tool.name,
    platform: platformAsStr(platform),
    risk_level: riskLevel,
    risk_reasons: tool.install_risk_reasons,
    warning: installWarningText(riskLevel),
    blocked: false,
    copy_gate: copyGate,
    command,
    config_json: configJson,
    copy_text: copyText,
    copy_label: copyLabel,
    steps,
    docs_links: docsLinksForTool(tool),
    x402_notice: x402NoticeForTool(tool),
    referral_disclosure: referralDisclosureForTool(tool),
  };
}