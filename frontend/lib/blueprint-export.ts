import { toPng } from "html-to-image";
import type { BlueprintEdge, BlueprintNode, PublicTool } from "@/lib/api";
import { buildFlowSection } from "@/lib/blueprint-flow-core.mjs";
import { SITE_ORIGIN } from "@/lib/site";

const AGENT_EXPORT_TASK_TEMPLATE = `## Your task

1. Read the attached blueprint image and this prompt together.
2. For each slug in ## Tools, call OnchainAI MCP \`get_install_guide\` (platform: cursor).
3. Summarize install risk; do not install critical-risk tools.
4. Follow ## Flow when proposing order; if I edited this section, prefer my wording.
5. Ask before changing my toolkit or installing anything.`;

type ExportNode = {
  id: string;
  kind: BlueprintNode["kind"];
  slug?: string;
  chainId?: string;
  text?: string;
  chains: string[];
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
  }));
}

export function buildDraftAgentMarkdown(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
  toolsBySlug: Record<string, PublicTool | null>,
): string {
  const exportNodes = parseExportNodes(nodes);
  const origin = clientSiteOrigin();
  let markdown = `# ${title}\n\n`;
  markdown +=
    "Read the attached blueprint image together with this prompt. " +
    "For each tool below, call OnchainAI MCP `get_install_guide` (platform: cursor) " +
    "before installing.\n\n";

  markdown += "## Tools\n\n";
  const toolNodes = exportNodes.filter((node) => node.kind === "tool");
  if (toolNodes.length === 0) {
    markdown += "(none)\n\n";
  } else {
    for (const node of toolNodes) {
      const slug = node.slug ?? "unknown";
      const displayName = toolsBySlug[slug]?.name ?? slug;
      const chains = node.chains.length > 0 ? node.chains.join(", ") : "none specified";
      markdown += `### ${displayName}\n`;
      markdown += `- Slug: \`${slug}\`\n`;
      markdown += `- Chains: ${chains}\n`;
      markdown += `- Page: ${origin}/tools/${slug}\n`;
      markdown += `- MCP: \`get_install_guide({ slug: "${slug}", platform: "cursor" })\`\n\n`;
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

  markdown += "## Flow\n\n";
  markdown += buildFlowSection(exportNodes, edges);
  markdown += "\n\n";
  markdown += AGENT_EXPORT_TASK_TEMPLATE;
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

export async function captureBlueprintViewport(viewportEl: HTMLElement): Promise<string> {
  viewportEl.setAttribute("data-blueprint-exporting", "true");
  try {
    const dataUrl = await toPng(viewportEl, {
      cacheBust: true,
      filter: shouldIncludeInCapture,
    });
    triggerPngDownload(dataUrl);
    return dataUrl;
  } finally {
    viewportEl.removeAttribute("data-blueprint-exporting");
  }
}