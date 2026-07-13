/** Install command helpers (shared by guide builders). */
import type { PublicTool } from "@/lib/api";
import { ADD_MCP_INTENT, stripAddModeParams } from "@/lib/browser-query";
import {
  type InstallSurfaceTool,
  type GuideLink,
  type PublicInstallGuide,
  blocksStructuredConfig,
} from "./install-guide-shared";

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

export function primaryInstallCommand(tool: InstallSurfaceTool): string | null {
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

export function toolHasInstallPath(tool: InstallSurfaceTool): boolean {
  return primaryInstallCommand(tool) !== null;
}

export function addMcpActionLabel(tool: InstallSurfaceTool): string | null {
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
  tool: PublicTool,
): Pick<PublicInstallGuide, "docs_links" | "x402_notice" | "referral_disclosure"> {
  return {
    docs_links: docsLinksForTool(tool),
    x402_notice: x402NoticeForTool(tool),
    referral_disclosure: referralDisclosureForTool(tool),
  };
}

function docsLinksForTool(tool: PublicTool): GuideLink[] {
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

function x402NoticeForTool(tool: PublicTool): string | null {
  if (tool.pricing !== "x402" && !tool.x402_price && !tool.referral_enabled) return null;
  const price = tool.x402_price?.trim() || "the provider's x402 price";
  const verification =
    tool.payment_verified && tool.x402_endpoint_verified && tool.price_verified
      ? "Payment details are operator verified."
      : "Payment details are not operator verified yet.";
  return `Calls may request x402 payment (${price}). OnchainAI discloses payment metadata only and does not connect wallets or process payments. ${verification}`;
}

function referralDisclosureForTool(tool: PublicTool): string | null {
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


