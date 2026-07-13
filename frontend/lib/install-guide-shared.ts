import type { InstallSurfaceTool, PublicTool } from "@/lib/api";
export type { InstallSurfaceTool };
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
  /** Shared install-guide types, labels, and pure helpers.
 * Phase 9: per-client install blocks (ChatGPT / Claude / Cursor / VS Code / More). */
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

