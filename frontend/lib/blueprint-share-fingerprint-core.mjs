/** @typedef {{ id: string; kind: string; slug?: string; chainId?: string; text?: string; chains?: string[]; step?: number; steps?: number[] }} BlueprintNodeLike */
/** @typedef {{ id: string; fromId: string; toId: string; style: string; color: string; dashed?: boolean; label?: string }} BlueprintEdgeLike */

/** FNV-1a 32-bit hash — short, deterministic fingerprint over canonical content. */
export function fnv1aHex(input) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return `fp:${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

/**
 * @param {string} title
 * @param {BlueprintNodeLike[]} nodes
 * @param {BlueprintEdgeLike[]} edges
 */
export function canonicalizeBlueprint(title, nodes, edges) {
  const sortedNodes = [...nodes].sort((a, b) => a.id.localeCompare(b.id));
  const nodeLines = sortedNodes.map((node) => {
    const parts = [node.id, node.kind];
    const slug = node.slug?.trim();
    if (slug) parts.push(`slug:${slug}`);
    const chainId = node.chainId?.trim();
    if (chainId) parts.push(`chainId:${chainId}`);
    if (node.text !== undefined) parts.push(`text:${node.text}`);
    if (node.chains?.length) parts.push(`chains:${[...node.chains].sort().join(",")}`);
    if (node.steps?.length) {
      parts.push(`steps:${[...node.steps].sort((a, b) => a - b).join(",")}`);
    } else if (node.step != null) {
      parts.push(`step:${node.step}`);
    }
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

/**
 * @param {string} title
 * @param {BlueprintNodeLike[]} nodes
 * @param {BlueprintEdgeLike[]} edges
 */
export function blueprintCanvasFingerprint(title, nodes, edges) {
  return fnv1aHex(canonicalizeBlueprint(title, nodes, edges));
}