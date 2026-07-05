import type { PublicTool } from "@/lib/api";
import { chainTagsForTool } from "@/lib/chains";
import { statusBadgeLabel, timeAgo, typeBadgeLabel } from "@/lib/format";

export const MIN_COMPARE_TOOLS = 2;
export const MAX_COMPARE_TOOLS = 4;

export type CompareRowKey =
  | "type"
  | "status"
  | "stars"
  | "license"
  | "updated"
  | "install_risk"
  | "chains"
  | "pricing";

export const COMPARE_ROWS: { key: CompareRowKey; label: string }[] = [
  { key: "type", label: "Type" },
  { key: "status", label: "Status" },
  { key: "stars", label: "GitHub stars" },
  { key: "license", label: "License" },
  { key: "updated", label: "Updated" },
  { key: "install_risk", label: "Install risk" },
  { key: "chains", label: "Chains" },
  { key: "pricing", label: "Pricing" },
];

export function normalizeCompareSlugs(raw: string): string[] {
  const seen = new Set<string>();
  return raw
    .split(",")
    .map((part) => {
      try {
        return decodeURIComponent(part.trim()).toLowerCase();
      } catch {
        return part.trim().toLowerCase();
      }
    })
    .filter((part) => part && !seen.has(part) && seen.add(part))
    .slice(0, MAX_COMPARE_TOOLS);
}

export function compareToolsQuery(slugs: string[]): string {
  return slugs.join(",");
}

function formatPricing(pricing: string): string {
  switch (pricing) {
    case "free":
      return "Free";
    case "paid":
      return "Paid";
    case "freemium":
      return "Freemium";
    case "x402":
      return "x402";
    default:
      return pricing || "—";
  }
}

function formatInstallRisk(level: string): string {
  switch (level) {
    case "low":
      return "Low";
    case "medium":
      return "Medium";
    case "high":
      return "High";
    case "critical":
      return "Critical";
    default:
      return level || "—";
  }
}

export function compareCellText(tool: PublicTool, key: CompareRowKey): string {
  switch (key) {
    case "type":
      return typeBadgeLabel(tool.type);
    case "status":
      return statusBadgeLabel(tool.status);
    case "stars":
      return String(tool.stars ?? 0);
    case "license":
      return tool.license?.trim() || "—";
    case "updated":
      return timeAgo(tool.last_commit_at || tool.updated_at);
    case "install_risk":
      return formatInstallRisk(tool.install_risk_level);
    case "pricing":
      return formatPricing(tool.pricing);
    case "chains": {
      const chains = chainTagsForTool(tool.chains);
      return chains.length > 0 ? chains.map((c) => c.label).join(", ") : "—";
    }
    default:
      return "—";
  }
}

export function rowValuesDiffer(tools: PublicTool[], key: CompareRowKey): boolean {
  if (tools.length < 2 || key === "chains") return false;
  const values = tools.map((tool) => compareCellText(tool, key).toLowerCase());
  return new Set(values).size > 1;
}

export function sharedChainIds(tools: PublicTool[]): Set<string> {
  if (tools.length < 2) return new Set();
  const chainSets = tools.map(
    (tool) => new Set(chainTagsForTool(tool.chains).map((chain) => chain.id)),
  );
  const [first, ...rest] = chainSets;
  const shared = new Set<string>();
  for (const id of first) {
    if (rest.every((set) => set.has(id))) {
      shared.add(id);
    }
  }
  return shared;
}