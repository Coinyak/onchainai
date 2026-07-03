export const BLUEPRINT_CANVAS_SIZE = 4000;
export const BLUEPRINT_GRID = 8;
export const BLUEPRINT_MAX_NODES = 120;

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