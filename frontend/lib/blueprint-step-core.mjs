/** @param {string} raw @param {number} maxStep */
export function parseBlueprintStepInput(raw, maxStep = 99) {
  const trimmed = raw.trim();
  if (!trimmed) return undefined;
  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isFinite(parsed) || parsed < 1) return undefined;
  return Math.min(maxStep, parsed);
}

const MAX_STEPS_PER_NODE = 8;

/**
 * Parse a multi-number step input like "#1 #7" or "1,7" or "1 7".
 * Returns a sorted, deduplicated array of valid step numbers (1..99).
 * @param {string} raw @param {number} maxStep
 * @returns {number[]}
 */
export function parseBlueprintStepsInput(raw, maxStep = 99) {
  const parts = raw.split(/[\s,]+/).map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return [];
  const seen = new Set();
  for (const part of parts) {
    const cleaned = part.replace(/^#/, "");
    const parsed = Number.parseInt(cleaned, 10);
    if (!Number.isFinite(parsed) || parsed < 1) continue;
    seen.add(Math.min(maxStep, parsed));
    if (seen.size >= MAX_STEPS_PER_NODE) break;
  }
  return [...seen].sort((a, b) => a - b);
}