/**
 * Pure helper for BlueprintShareDock auto-regenerate gating (issue #49).
 * @param {{
 *   open: boolean;
 *   hasNodes: boolean;
 *   loading: boolean;
 *   baselineFingerprint: string;
 *   canvasFingerprint: string;
 *   isDirty: boolean;
 * }} opts
 */
export function shouldAutoRegenSharePrompt(opts) {
  if (!opts.open || !opts.hasNodes || opts.loading) return false;
  if (!opts.baselineFingerprint) return false;
  if (opts.canvasFingerprint === opts.baselineFingerprint) return false;
  if (opts.isDirty) return false;
  return true;
}

/**
 * @param {boolean} isCanvasStale
 * @param {boolean} isDirty
 */
export function shouldShowStaleShareBanner(isCanvasStale, isDirty) {
  return isCanvasStale && isDirty;
}