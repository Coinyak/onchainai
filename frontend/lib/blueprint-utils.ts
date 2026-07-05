import type { BlueprintEdge, BlueprintNode } from "@/lib/api";
import { chainTagsForTool, type ChainMeta } from "@/lib/chains";

export const BLUEPRINT_CANVAS_SIZE = 4000;
export const BLUEPRINT_GRID = 8;
export const BLUEPRINT_MAX_NODES = 120;
export const BLUEPRINT_MAX_EDGES = 120;
export const BLUEPRINT_MAX_TOOL_CHAINS = 8;

export type BlueprintEdgeStyle = "solid" | "arrow";

export const BLUEPRINT_EDGE_COLORS = [
  { id: "neutral", label: "Neutral", value: "#1A1A1A" },
  { id: "orange", label: "Orange", value: "#E76F00" },
  { id: "blue", label: "Blue", value: "#2B6CB0" },
  { id: "green", label: "Green", value: "#2D7D46" },
  { id: "purple", label: "Purple", value: "#5C6BC0" },
] as const;

export const BLUEPRINT_NODE_TOOL_WIDTH = 260;
export const BLUEPRINT_NODE_TOOL_HEIGHT = 112;
export const BLUEPRINT_NODE_NOTE_WIDTH = 260;
export const BLUEPRINT_NODE_NOTE_HEIGHT = 88;
export const BLUEPRINT_NODE_CHAIN_SIZE = 48;
export const BLUEPRINT_NODE_CHAIN_WRAP_HEIGHT = 66;

// Custom card size bounds for tool/note nodes (kept in sync with the backend
// clamp in src/server/api_v2/blueprints.rs). Chain stickers are fixed-size.
export const BLUEPRINT_NODE_MIN_W = 160;
export const BLUEPRINT_NODE_MAX_W = 520;
export const BLUEPRINT_NODE_MIN_H = 72;
export const BLUEPRINT_NODE_MAX_H = 420;
// Height thresholds below which optional tool rows collapse so text never clips.
export const BLUEPRINT_NODE_TOOL_TYPE_MIN_H = 88;
export const BLUEPRINT_NODE_TOOL_CHAINS_MIN_H = 112;
export const BLUEPRINT_NODE_MAX_STEP = 99;

export function clampNodeWidth(w: number): number {
  return Math.max(BLUEPRINT_NODE_MIN_W, Math.min(BLUEPRINT_NODE_MAX_W, Math.round(w)));
}

export function clampNodeHeight(h: number): number {
  return Math.max(BLUEPRINT_NODE_MIN_H, Math.min(BLUEPRINT_NODE_MAX_H, Math.round(h)));
}

/** Mirrors Rust `chars().take(n)` for edge label normalization. */
export function truncateBlueprintLabel(label: string, maxChars = 40): string {
  return [...label].slice(0, maxChars).join("");
}

export type BlueprintPortSide = "top" | "right" | "bottom" | "left";

/** Which end of an existing edge is being dragged during a reconnect. */
export type BlueprintEndpoint = "from" | "to";

export function snapToGrid(value: number, grid = BLUEPRINT_GRID): number {
  return Math.round(value / grid) * grid;
}

export function clampCoord(value: number): number {
  return Math.max(0, Math.min(BLUEPRINT_CANVAS_SIZE, snapToGrid(value)));
}

export function newNodeId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `node-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export function newEdgeId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `edge-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export function getNodeBounds(node: BlueprintNode): {
  x: number;
  y: number;
  w: number;
  h: number;
} {
  if (node.kind === "chain") {
    return {
      x: node.x,
      y: node.y,
      w: BLUEPRINT_NODE_CHAIN_SIZE,
      h: BLUEPRINT_NODE_CHAIN_WRAP_HEIGHT,
    };
  }
  const defaultW =
    node.kind === "note" ? BLUEPRINT_NODE_NOTE_WIDTH : BLUEPRINT_NODE_TOOL_WIDTH;
  const defaultH =
    node.kind === "note" ? BLUEPRINT_NODE_NOTE_HEIGHT : BLUEPRINT_NODE_TOOL_HEIGHT;
  return {
    x: node.x,
    y: node.y,
    w: node.w != null ? clampNodeWidth(node.w) : defaultW,
    h: node.h != null ? clampNodeHeight(node.h) : defaultH,
  };
}

export function getNodeAnchor(
  node: BlueprintNode,
  side: BlueprintPortSide,
): { x: number; y: number } {
  const bounds = getNodeBounds(node);
  const centerX = bounds.x + bounds.w / 2;
  const centerY = bounds.y + bounds.h / 2;
  switch (side) {
    case "top":
      return { x: centerX, y: bounds.y };
    case "right":
      return { x: bounds.x + bounds.w, y: centerY };
    case "bottom":
      return { x: centerX, y: bounds.y + bounds.h };
    case "left":
      return { x: bounds.x, y: centerY };
  }
}

export function pickEdgePortSides(
  from: BlueprintNode,
  to: BlueprintNode,
): { from: BlueprintPortSide; to: BlueprintPortSide } {
  const fromBounds = getNodeBounds(from);
  const toBounds = getNodeBounds(to);
  const fromCx = fromBounds.x + fromBounds.w / 2;
  const fromCy = fromBounds.y + fromBounds.h / 2;
  const toCx = toBounds.x + toBounds.w / 2;
  const toCy = toBounds.y + toBounds.h / 2;
  const dx = toCx - fromCx;
  const dy = toCy - fromCy;

  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx >= 0
      ? { from: "right", to: "left" }
      : { from: "left", to: "right" };
  }
  return dy >= 0
    ? { from: "bottom", to: "top" }
    : { from: "top", to: "bottom" };
}

export function buildEdgePath(
  from: BlueprintNode,
  to: BlueprintNode,
): { x1: number; y1: number; x2: number; y2: number } {
  const { from: fromSide, to: toSide } = pickEdgePortSides(from, to);
  const start = getNodeAnchor(from, fromSide);
  const end = getNodeAnchor(to, toSide);
  return { x1: start.x, y1: start.y, x2: end.x, y2: end.y };
}

export function toolChainsForNode(toolChains: string[]): ChainMeta[] {
  return chainTagsForTool(toolChains);
}

export function initialToolNodeChains(toolChains: string[]): string[] {
  const available = toolChainsForNode(toolChains);
  if (available.length === 1) return [available[0].id];
  return [];
}

export function normalizeToolNodeChains(
  selected: string[],
  toolChains: string[],
): string[] {
  const available = new Set(toolChainsForNode(toolChains).map((chain) => chain.id));
  const seen = new Set<string>();
  const normalized: string[] = [];
  for (const chainId of selected) {
    const id = chainId.trim().toLowerCase();
    if (!id || !available.has(id) || seen.has(id)) continue;
    seen.add(id);
    normalized.push(id);
    if (normalized.length >= BLUEPRINT_MAX_TOOL_CHAINS) break;
  }
  return normalized;
}

export function pruneEdgesForNodes(
  edges: BlueprintEdge[],
  nodes: BlueprintNode[],
): BlueprintEdge[] {
  const nodeIds = new Set(nodes.map((node) => node.id));
  return edges.filter(
    (edge) => nodeIds.has(edge.fromId) && nodeIds.has(edge.toId),
  );
}

export function pointerToCanvasCoords(
  clientX: number,
  clientY: number,
  viewportEl: HTMLElement,
): { x: number; y: number } {
  const rect = viewportEl.getBoundingClientRect();
  const rawX = viewportEl.scrollLeft + (clientX - rect.left);
  const rawY = viewportEl.scrollTop + (clientY - rect.top);
  return {
    x: clampCoord(rawX),
    y: clampCoord(rawY),
  };
}