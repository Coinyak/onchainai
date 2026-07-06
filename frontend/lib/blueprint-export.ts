import { toPng } from "html-to-image";
import type { BlueprintEdge, BlueprintNode, PublicTool } from "@/lib/api";
import { buildFlowSection } from "@/lib/blueprint-flow-core.mjs";
import { SITE_ORIGIN } from "@/lib/site";

export type BlueprintExportPlatform =
  | "generic"
  | "claude"
  | "cursor"
  | "vscode"
  | "chatgpt"
  | "codex"
  | "gemini"
  | "windsurf";

export interface BlueprintExportPlatformMeta {
  id: BlueprintExportPlatform;
  label: string;
  mcpPlatform: string;
  logoId: string;
}

export const BLUEPRINT_EXPORT_PLATFORMS: BlueprintExportPlatformMeta[] = [
  { id: "generic", label: "Generic", mcpPlatform: "generic", logoId: "generic" },
  { id: "claude", label: "Claude", mcpPlatform: "claude", logoId: "claude" },
  { id: "cursor", label: "Cursor", mcpPlatform: "cursor", logoId: "cursor" },
  { id: "vscode", label: "VS Code", mcpPlatform: "generic", logoId: "vscode" },
  { id: "chatgpt", label: "ChatGPT", mcpPlatform: "generic", logoId: "openai" },
  { id: "codex", label: "Codex CLI", mcpPlatform: "generic", logoId: "openai" },
  { id: "gemini", label: "Gemini", mcpPlatform: "generic", logoId: "gemini" },
  { id: "windsurf", label: "Windsurf", mcpPlatform: "generic", logoId: "windsurf" },
];

export const DEFAULT_EXPORT_PLATFORM: BlueprintExportPlatform = "generic";

const AGENT_EXPORT_TASK_TEMPLATE = `## Your task

1. Read the attached blueprint PNG together with this prompt (export PNG separately from the editor Share dock).
2. For each slug in ## Tools, call OnchainAI MCP \`get_install_guide\` (platform: {platform}).
3. Summarize install risk; do not install critical-risk tools.
4. When ## Order is present, treat it as the owner's step sequence; otherwise follow ## Flow. If you edited Flow/Order, prefer the user's wording.
5. Ask before changing my toolkit or installing anything.`;

type ExportNode = {
  id: string;
  kind: BlueprintNode["kind"];
  slug?: string;
  chainId?: string;
  text?: string;
  chains: string[];
  steps: number[];
};

function clientSiteOrigin(): string {
  if (typeof window !== "undefined") return window.location.origin;
  return SITE_ORIGIN;
}

function parseExportNodes(nodes: BlueprintNode[]): ExportNode[] {
  return nodes.map((node) => ({
    id: node.id,
    kind: node.kind,
    slug: node.slug,
    chainId: node.chainId,
    text: node.text,
    chains: node.chains ?? [],
    steps: node.steps ?? (node.step != null ? [node.step] : []),
  }));
}

function nodeFlowLabel(node: ExportNode): string {
  if (node.kind === "tool") return node.slug ?? node.id;
  if (node.kind === "note") {
    const text = node.text?.trim() ?? "";
    if (!text) return "note";
    const chars = [...text];
    if (chars.length > 48) return `note: ${chars.slice(0, 48).join("")}…`;
    return `note: ${text}`;
  }
  if (node.kind === "chain") return `chain: ${node.chainId ?? "unknown"}`;
  return node.id;
}

/** Mirrors Rust build_order_section. */
export function buildOrderSection(exportNodes: ExportNode[]): string {
  // Expand each node's steps into (node, step) pairs, then sort globally.
  const stepped: { node: ExportNode; step: number }[] = [];
  for (const node of exportNodes) {
    for (const step of node.steps) {
      stepped.push({ node, step });
    }
  }
  if (stepped.length === 0) return "";

  stepped.sort((a, b) => a.step - b.step);
  return stepped
    .map(({ node, step }) => `- ${step}. ${nodeFlowLabel(node)} (${node.kind})`)
    .join("\n");
}

export function buildDraftAgentMarkdown(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
  toolsBySlug: Record<string, PublicTool | null>,
  platform: BlueprintExportPlatform = DEFAULT_EXPORT_PLATFORM,
): string {
  const exportNodes = parseExportNodes(nodes);
  const origin = clientSiteOrigin();
  const mcpPlatform = BLUEPRINT_EXPORT_PLATFORMS.find((p) => p.id === platform)?.mcpPlatform ?? "generic";
  let markdown = `# ${title}\n\n`;
  markdown +=
    "Read the attached blueprint PNG together with this prompt (export PNG from the Share dock Image tab). " +
    `For each tool below, call OnchainAI MCP \`get_install_guide\` (platform: ${mcpPlatform}) ` +
    "before installing.\n\n";

  markdown += "## Tools\n\n";
  const toolNodes = exportNodes
    .map((node, index) => ({ node, index }))
    .filter(({ node }) => node.kind === "tool");
  toolNodes.sort((a, b) => {
    const aMin = a.node.steps.length > 0 ? Math.min(...a.node.steps) : null;
    const bMin = b.node.steps.length > 0 ? Math.min(...b.node.steps) : null;
    if (aMin != null && bMin != null) return aMin - bMin;
    if (aMin != null) return -1;
    if (bMin != null) return 1;
    return a.index - b.index;
  });

  if (toolNodes.length === 0) {
    markdown += "(none)\n\n";
  } else {
    for (const { node } of toolNodes) {
      const slug = node.slug ?? "unknown";
      const displayName = toolsBySlug[slug]?.name ?? slug;
      const chains = node.chains.length > 0 ? node.chains.join(", ") : "none specified";
      const stepBadges = node.steps.length > 0
        ? ` #${node.steps.sort((a, b) => a - b).join(" #")}`
        : "";
      markdown += `### ${displayName}${stepBadges}\n`;
      markdown += `- Slug: \`${slug}\`\n`;
      markdown += `- Chains: ${chains}\n`;
      const installRisk = toolsBySlug[slug]?.install_risk_level;
      if (installRisk) {
        markdown += `- Install risk: ${installRisk}\n`;
      }
      markdown += `- Page: ${origin}/tools/${slug}\n`;
      markdown += `- MCP: \`get_install_guide({ slug: "${slug}", platform: "${mcpPlatform}" })\`\n\n`;
    }
  }

  markdown += "## Notes\n\n";
  const noteTexts = exportNodes
    .filter((node) => node.kind === "note")
    .map((node) => node.text?.trim() ?? "")
    .filter((text) => text.length > 0);
  if (noteTexts.length === 0) {
    markdown += "(none)\n\n";
  } else {
    for (const text of noteTexts) {
      markdown += `- ${text}\n`;
    }
    markdown += "\n";
  }

  const orderSection = buildOrderSection(exportNodes);
  if (orderSection) {
    markdown += "## Order\n\n";
    markdown += orderSection;
    markdown += "\n\n";
  }

  markdown += "## Flow\n\n";
  markdown += buildFlowSection(exportNodes, edges);
  markdown += "\n\n";
  markdown += AGENT_EXPORT_TASK_TEMPLATE.replace("{platform}", mcpPlatform);
  return markdown;
}

function shouldIncludeInCapture(node: Node): boolean {
  if (!(node instanceof HTMLElement)) return true;
  return !node.classList.contains("blueprint-share-dock") && !node.closest(".blueprint-share-dock");
}

function triggerPngDownload(dataUrl: string, filename = "blueprint.png"): void {
  const link = document.createElement("a");
  link.download = filename;
  link.href = dataUrl;
  link.click();
}

async function captureElementPng(
  viewportEl: HTMLElement,
  targetEl: HTMLElement,
): Promise<string> {
  viewportEl.setAttribute("data-blueprint-exporting", "true");
  try {
    const dataUrl = await toPng(targetEl, {
      cacheBust: true,
      filter: shouldIncludeInCapture,
    });
    triggerPngDownload(dataUrl);
    return dataUrl;
  } finally {
    viewportEl.removeAttribute("data-blueprint-exporting");
  }
}

/** Captures the visible viewport area (what is currently on screen). */
export async function captureBlueprintViewport(viewportEl: HTMLElement): Promise<string> {
  return captureElementPng(viewportEl, viewportEl);
}

/** Captures the full `.blueprint-canvas-surface` inside the viewport. */
export async function captureBlueprintContent(viewportEl: HTMLElement): Promise<string> {
  const surfaceEl = viewportEl.querySelector<HTMLElement>(".blueprint-canvas-surface");
  if (!surfaceEl) {
    throw new Error("Blueprint canvas surface not found");
  }
  return captureElementPng(viewportEl, surfaceEl);
}