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

function stableStringify(value: unknown): string {
  return JSON.stringify(sortKeys(value));
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortKeys);
  }
  if (value && typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(obj).sort()) {
      sorted[key] = sortKeys(obj[key]);
    }
    return sorted;
  }
  return value;
}

/** Stable fingerprint of blueprint content for share-prompt draft invalidation. */
export function blueprintCanvasFingerprint(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
): string {
  const payload = {
    title: title.trim(),
    nodes: nodes.map((node) => {
      const normalized: Record<string, unknown> = {
        id: node.id,
        kind: node.kind,
      };
      if (node.slug?.trim()) normalized.slug = node.slug.trim();
      if (node.chainId?.trim()) normalized.chainId = node.chainId.trim();
      if (node.text !== undefined) normalized.text = node.text;
      if (node.chains?.length) normalized.chains = [...node.chains].sort();
      if (node.step != null) normalized.step = node.step;
      return normalized;
    }),
    edges: edges.map((edge) => {
      const normalized: Record<string, unknown> = {
        id: edge.id,
        fromId: edge.fromId,
        toId: edge.toId,
        style: edge.style,
        color: edge.color,
      };
      if (edge.dashed) normalized.dashed = true;
      const label = edge.label?.trim();
      if (label) normalized.label = label;
      return normalized;
    }),
  };
  return stableStringify(payload);
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