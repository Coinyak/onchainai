import type { BlueprintNode } from "@/lib/api";

export const BLUEPRINT_DRAFT_KEY = "onchainai-blueprint-draft";
export const BLUEPRINT_DRAFT_ID = "draft";

export interface LocalBlueprintDraft {
  title: string;
  nodes: BlueprintNode[];
  updatedAt: string;
}

let cachedRaw: string | null | undefined;
let cachedSnapshot: LocalBlueprintDraft | null = null;

export function loadLocalBlueprintDraft(): LocalBlueprintDraft | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(BLUEPRINT_DRAFT_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as LocalBlueprintDraft;
    if (!parsed || typeof parsed.title !== "string" || !Array.isArray(parsed.nodes)) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function saveLocalBlueprintDraft(draft: LocalBlueprintDraft): void {
  if (typeof window === "undefined") return;
  try {
    const raw = JSON.stringify(draft);
    window.localStorage.setItem(BLUEPRINT_DRAFT_KEY, raw);
    cachedRaw = raw;
    cachedSnapshot = draft;
  } catch {
    return;
  }
  window.dispatchEvent(new Event("blueprint-draft-change"));
}

export function subscribeLocalBlueprintDraft(onChange: () => void): () => void {
  const handler = () => onChange();
  window.addEventListener("blueprint-draft-change", handler);
  window.addEventListener("storage", handler);
  return () => {
    window.removeEventListener("blueprint-draft-change", handler);
    window.removeEventListener("storage", handler);
  };
}

export function getLocalBlueprintDraftSnapshot(): LocalBlueprintDraft | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(BLUEPRINT_DRAFT_KEY);
  if (raw === cachedRaw) return cachedSnapshot;
  cachedRaw = raw;
  cachedSnapshot = loadLocalBlueprintDraft();
  return cachedSnapshot;
}

export function clearLocalBlueprintDraft(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(BLUEPRINT_DRAFT_KEY);
  cachedRaw = null;
  cachedSnapshot = null;
}

export function createEmptyDraft(): LocalBlueprintDraft {
  return {
    title: "Untitled blueprint",
    nodes: [],
    updatedAt: new Date().toISOString(),
  };
}