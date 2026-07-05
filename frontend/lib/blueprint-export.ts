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

interface FlowEdge {
  from: string;
  to: string;
  label?: string;
}

const cmpStr = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);

/**
 * Follow a maximal simple path from `startIdx`, stopping at branch/merge points
 * (nodes whose in- or out-degree is not exactly 1) or a revisited edge. Mirrors
 * the Rust `walk_flow_segment` so drafts and saved blueprints read identically.
 */
function walkFlowSegment(
  startIdx: number,
  flowEdges: FlowEdge[],
  outEdges: Map<string, number[]>,
  inDeg: Map<string, number>,
  outDeg: Map<string, number>,
  labelOf: (id: string) => string,
  visited: boolean[],
): string {
  let line = labelOf(flowEdges[startIdx].from);
  let cur = startIdx;
  for (;;) {
    visited[cur] = true;
    const edge = flowEdges[cur];
    line += edge.label ? ` →(${edge.label}) ` : " → ";
    line += labelOf(edge.to);

    const internal =
      (inDeg.get(edge.to) ?? 0) === 1 && (outDeg.get(edge.to) ?? 0) === 1;
    if (internal) {
      const next = outEdges.get(edge.to)?.[0];
      if (next !== undefined && !visited[next]) {
        cur = next;
        continue;
      }
    }
    break;
  }
  return line;
}

function buildFlowSection(nodes: ExportNode[], edges: BlueprintEdge[]): string {
  const nodeMap = new Map(nodes.map((node) => [node.id, node]));
  const labelOf = (id: string): string => {
    const node = nodeMap.get(id);
    return node ? nodeFlowLabel(node) : id;
  };

  const flowEdges: FlowEdge[] = [];
  for (const edge of edges) {
    const from = edge.fromId?.trim() ?? "";
    const to = edge.toId?.trim() ?? "";
    if (!from || !to) continue;
    if (!nodeMap.has(from) || !nodeMap.has(to)) continue;
    const label = edge.label?.trim();
    flowEdges.push({ from, to, label: label ? label : undefined });
  }

  if (flowEdges.length === 0) return "(no flow edges defined)";

  const inDeg = new Map<string, number>();
  const outDeg = new Map<string, number>();
  const outEdges = new Map<string, number[]>();
  for (const node of nodes) {
    inDeg.set(node.id, 0);
    outDeg.set(node.id, 0);
  }
  flowEdges.forEach((edge, idx) => {
    outDeg.set(edge.from, (outDeg.get(edge.from) ?? 0) + 1);
    inDeg.set(edge.to, (inDeg.get(edge.to) ?? 0) + 1);
    if (!inDeg.has(edge.from)) inDeg.set(edge.from, 0);
    if (!outDeg.has(edge.to)) outDeg.set(edge.to, 0);
    const list = outEdges.get(edge.from) ?? [];
    list.push(idx);
    outEdges.set(edge.from, list);
  });

  // Deterministic output: sort each node's out-edges by their target label.
  for (const list of outEdges.values()) {
    list.sort((a, b) => cmpStr(labelOf(flowEdges[a].to), labelOf(flowEdges[b].to)));
  }

  const visited: boolean[] = new Array(flowEdges.length).fill(false);
  const lines: string[] = [];

  // 1. Segments that begin at a junction (source/sink/branch/merge).
  const junctions = [...outEdges.keys()].sort(
    (a, b) => cmpStr(labelOf(a), labelOf(b)) || cmpStr(a, b),
  );
  for (const from of junctions) {
    const isJunction =
      (inDeg.get(from) ?? 0) !== 1 || (outDeg.get(from) ?? 0) !== 1;
    if (!isJunction) continue;
    for (const idx of outEdges.get(from) ?? []) {
      if (!visited[idx]) {
        lines.push(
          walkFlowSegment(idx, flowEdges, outEdges, inDeg, outDeg, labelOf, visited),
        );
      }
    }
  }

  // 2. Remaining edges belong to pure cycles with no junction — emit them too.
  for (let idx = 0; idx < flowEdges.length; idx += 1) {
    if (!visited[idx]) {
      lines.push(
        walkFlowSegment(idx, flowEdges, outEdges, inDeg, outDeg, labelOf, visited),
      );
    }
  }

  // 3. Nodes touching no edge, listed on their own for completeness.
  const touched = new Set(flowEdges.flatMap((edge) => [edge.from, edge.to]));
  const orphans = nodes
    .filter((node) => !touched.has(node.id))
    .map(nodeFlowLabel)
    .sort(cmpStr);
  lines.push(...orphans);

  if (lines.length === 0) return "(no flow edges defined)";
  return lines.map((line) => `- ${line}`).join("\n");
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