import { toPng } from "html-to-image";
import type { BlueprintEdge, BlueprintNode, PublicTool } from "@/lib/api";
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

function nodeFlowLabel(node: ExportNode): string {
  if (node.kind === "tool") {
    return node.slug ?? node.id;
  }
  if (node.kind === "note") {
    const text = node.text?.trim() ?? "";
    if (!text) return "note";
    const chars = [...text];
    if (chars.length > 48) {
      return `note: ${chars.slice(0, 48).join("")}…`;
    }
    return `note: ${text}`;
  }
  if (node.kind === "chain") {
    return `chain: ${node.chainId ?? "unknown"}`;
  }
  return node.id;
}

function buildFlowSection(nodes: ExportNode[], edges: BlueprintEdge[]): string {
  const nodeMap = new Map(nodes.map((node) => [node.id, node]));
  const adj = new Map<string, string[]>();
  const inDegree = new Map<string, number>();
  const edgePairs: Array<[string, string]> = [];

  for (const node of nodes) {
    inDegree.set(node.id, 0);
  }

  for (const edge of edges) {
    const fromId = edge.fromId.trim();
    const toId = edge.toId.trim();
    if (!fromId || !toId) continue;
    if (!nodeMap.has(fromId) || !nodeMap.has(toId)) continue;

    const neighbors = adj.get(fromId) ?? [];
    neighbors.push(toId);
    adj.set(fromId, neighbors);
    inDegree.set(toId, (inDegree.get(toId) ?? 0) + 1);
    inDegree.set(fromId, inDegree.get(fromId) ?? 0);
    edgePairs.push([fromId, toId]);
  }

  if (edgePairs.length === 0) {
    return "(no flow edges defined)";
  }

  const queue = [...inDegree.entries()]
    .filter(([, degree]) => degree === 0)
    .map(([id]) => id)
    .sort();

  const sorted: string[] = [];
  while (queue.length > 0) {
    const id = queue.shift();
    if (!id) break;
    sorted.push(id);

    const neighbors = [...(adj.get(id) ?? [])].sort();
    for (const next of neighbors) {
      const degree = (inDegree.get(next) ?? 0) - 1;
      inDegree.set(next, degree);
      if (degree === 0) {
        queue.push(next);
        queue.sort();
      }
    }
  }

  const nodesInEdges = new Set(edgePairs.flatMap(([from, to]) => [from, to]));

  if (sorted.length < nodesInEdges.size) {
    return edgePairs
      .map(([from, to]) => {
        const fromLabel = nodeMap.get(from) ? nodeFlowLabel(nodeMap.get(from)!) : from;
        const toLabel = nodeMap.get(to) ? nodeFlowLabel(nodeMap.get(to)!) : to;
        return `- ${fromLabel} → ${toLabel}`;
      })
      .join("\n");
  }

  const sortedSet = new Set(sorted);
  const labels = sorted
    .map((id) => nodeMap.get(id))
    .filter((node): node is ExportNode => !!node)
    .map(nodeFlowLabel);

  const orphans = nodes
    .filter((node) => !sortedSet.has(node.id) && !nodesInEdges.has(node.id))
    .map(nodeFlowLabel)
    .sort();
  labels.push(...orphans);

  if (labels.length === 0) return "(no flow edges defined)";
  if (labels.length === 1) return `- ${labels[0]}`;
  return `- ${labels.join(" → ")}`;
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