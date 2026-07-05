/** @param {string} raw @param {number} maxStep */
export function parseBlueprintStepInput(raw, maxStep = 99) {
  const trimmed = raw.trim();
  if (!trimmed) return undefined;
  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isFinite(parsed) || parsed < 1) return undefined;
  return Math.min(maxStep, parsed);
}