import type { CategoryWithCount } from "@/lib/api";
import { type BrowserQueryParams, parseMulti } from "@/lib/browser-query";

type LabeledOption = { id: string; label: string };

const ASSET_CLASS_LABELS: LabeledOption[] = [
  { id: "crypto", label: "Crypto" },
  { id: "stablecoins", label: "Stablecoins" },
  { id: "derivatives", label: "Derivatives" },
  { id: "rwa", label: "RWA" },
];

const ACTOR_LABELS: LabeledOption[] = [
  { id: "human", label: "Human" },
  { id: "ai-agent", label: "AI Agent" },
];

const TYPE_LABELS: LabeledOption[] = [
  { id: "mcp", label: "MCP" },
  { id: "cli", label: "CLI" },
  { id: "sdk", label: "SDK" },
  { id: "api", label: "API" },
  { id: "x402", label: "x402" },
  { id: "skill", label: "Skill" },
];

const STATUS_LABELS: LabeledOption[] = [
  { id: "community", label: "Community" },
  { id: "verified", label: "Verified" },
  { id: "official", label: "Official" },
];

const PRICING_LABELS: LabeledOption[] = [
  { id: "free", label: "Free" },
  { id: "x402", label: "x402" },
  { id: "paid", label: "Paid" },
  { id: "freemium", label: "Freemium" },
];

const INSTALL_RISK_LABELS: LabeledOption[] = [
  { id: "low", label: "Low" },
  { id: "medium", label: "Medium" },
  { id: "high", label: "High" },
];

function labelFor(id: string, options: LabeledOption[]): string {
  return options.find((opt) => opt.id === id)?.label ?? id;
}

function functionLabel(id: string, categories: CategoryWithCount[]): string {
  const row = categories.find((c) => c.category.id === id);
  if (!row) return id;
  const short = row.category.label.split(" &")[0]?.trim();
  return short || row.category.label;
}

function pushAxisLines(
  lines: string[],
  prefix: string,
  values: string[],
  options: LabeledOption[],
) {
  for (const id of values) {
    lines.push(`${prefix}: ${labelFor(id, options)}`);
  }
}

export function describeActiveFilters(
  params: BrowserQueryParams,
  categories: CategoryWithCount[] = [],
): string[] {
  const lines: string[] = [];

  for (const id of parseMulti(params.function)) {
    lines.push(`Function: ${functionLabel(id, categories)}`);
  }
  pushAxisLines(lines, "Asset class", parseMulti(params.asset_class), ASSET_CLASS_LABELS);
  pushAxisLines(lines, "Actor", parseMulti(params.actor), ACTOR_LABELS);
  pushAxisLines(lines, "Type", parseMulti(params.type), TYPE_LABELS);
  pushAxisLines(lines, "Status", parseMulti(params.status), STATUS_LABELS);
  pushAxisLines(lines, "Pricing", parseMulti(params.pricing), PRICING_LABELS);
  pushAxisLines(lines, "Install risk", parseMulti(params.install_risk), INSTALL_RISK_LABELS);

  for (const id of parseMulti(params.chain)) {
    const chainLabel = id.charAt(0).toUpperCase() + id.slice(1);
    lines.push(`Chain: ${chainLabel}`);
  }

  const q = params.q?.trim();
  if (q) lines.push(`Search: "${q}"`);

  return lines;
}

/** J1: explain zero intersection when the function still has tools in other types. */
export function buildEmptyIntersectionMessage(
  params: BrowserQueryParams,
  categories: CategoryWithCount[],
): string | undefined {
  const functions = parseMulti(params.function);
  const types = parseMulti(params.type);
  if (functions.length !== 1 || types.length !== 1) return undefined;

  const row = categories.find((c) => c.category.id === functions[0]);
  if (!row || row.count <= 0) return undefined;

  const fnName = functionLabel(functions[0], categories);
  const typeName = labelFor(types[0], TYPE_LABELS);
  return `No ${fnName} + ${typeName} tools yet. ${row.count} ${fnName} tools exist in other types.`;
}