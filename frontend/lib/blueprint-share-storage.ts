import type { BlueprintEdge, BlueprintNode } from "@/lib/api";
import { blueprintCanvasFingerprint as fingerprintFromCore } from "./blueprint-share-fingerprint-core.mjs";

const SHARE_DRAFT_KEY_PREFIX = "onchainai-blueprint-share-draft:";

export interface SharePromptDraft {
  fingerprint: string;
  markdown: string;
  savedAt: string;
}

function shareDraftKey(blueprintId: string): string {
  return `${SHARE_DRAFT_KEY_PREFIX}${blueprintId}`;
}

/** Stable fingerprint of blueprint content for share-prompt draft invalidation. */
export function blueprintCanvasFingerprint(
  title: string,
  nodes: BlueprintNode[],
  edges: BlueprintEdge[],
): string {
  return fingerprintFromCore(title, nodes, edges);
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