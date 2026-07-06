import type { BlueprintEdge, BlueprintNode } from "@/lib/api";

const SHARE_DRAFT_KEY_PREFIX = "onchainai-blueprint-share-draft:";

export interface SharePromptDraft {
  fingerprint: string;
  markdown: string;
  savedAt: string;
}

function shareDraftKey(blueprintId: string): string {
  return `${SHARE_DRAFT_KEY_PREFIX}${blueprintId}`;
}

/** FNV-1a 32-bit hash — short, deterministic fingerprint over canonical content. */
function fnv1aHex(input: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return `fp:${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

function canonicalizeBlueprint(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
): string {
  const sortedNodes = [...nodes].sort((a, b) => a.id.localeCompare(b.id));
  const nodeLines = sortedNodes.map((node) => {
    const parts = [node.id, node.kind];
    const slug = node.slug?.trim();
    if (slug) parts.push(`slug:${slug}`);
    const chainId = node.chainId?.trim();
    if (chainId) parts.push(`chainId:${chainId}`);
    if (node.text !== undefined) parts.push(`text:${node.text}`);
    if (node.chains?.length) parts.push(`chains:${[...node.chains].sort().join(",")}`);
    if (node.steps?.length) parts.push(`steps:${[...node.steps].sort((a, b) => a - b).join(",")}`);
    else if (node.step != null) parts.push(`step:${node.step}`);
    return parts.join("|");
  });

  const sortedEdges = [...edges].sort((a, b) => a.id.localeCompare(b.id));
  const edgeLines = sortedEdges.map((edge) => {
    const parts = [edge.id, edge.fromId, edge.toId, edge.style, edge.color];
    if (edge.dashed) parts.push("dashed");
    const label = edge.label?.trim();
    if (label) parts.push(`label:${label}`);
    return parts.join("|");
  });

  return [title.trim(), nodeLines.join("\n"), edgeLines.join("\n")].join("\x1f");
}

/** Stable fingerprint of blueprint content for share-prompt draft invalidation. */
export function blueprintCanvasFingerprint(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
): string {
  return fnv1aHex(canonicalizeBlueprint(title, nodes, edges));
}

export function loadSharePromptDraft(blueprintId: string): SharePromptDraft | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(shareDraftKey(blueprintId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as SharePromptDraft;
    if (
      !parsed ||
      typeof parsed.fingerprint !== "string" ||
      typeof parsed.markdown !== "string"
    ) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function saveSharePromptDraft(
  blueprintId: string,
  fingerprint: string,
  markdown: string,
): void {
  if (typeof window === "undefined") return;
  try {
    const draft: SharePromptDraft = {
      fingerprint,
      markdown,
      savedAt: new Date().toISOString(),
    };
    window.localStorage.setItem(shareDraftKey(blueprintId), JSON.stringify(draft));
  } catch {
    return;
  }
}

export function clearSharePromptDraft(blueprintId: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(shareDraftKey(blueprintId));
  } catch {
    return;
  }
}