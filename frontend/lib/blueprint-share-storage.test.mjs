import assert from "node:assert/strict";
import test from "node:test";

// Minimal fingerprint mirror for node tests (TS module not imported in node --test).
function fnv1aHex(input) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return `fp:${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

function canonicalizeBlueprint(title, nodes, edges) {
  const sortedNodes = [...nodes].sort((a, b) => a.id.localeCompare(b.id));
  const nodeLines = sortedNodes.map((node) => {
    const parts = [node.id, node.kind];
    if (node.slug?.trim()) parts.push(`slug:${node.slug.trim()}`);
    if (node.step != null) parts.push(`step:${node.step}`);
    if (node.steps?.length) parts.push(`steps:${[...node.steps].sort((a, b) => a - b).join(",")}`);
    return parts.join("|");
  });
  const sortedEdges = [...edges].sort((a, b) => a.id.localeCompare(b.id));
  const edgeLines = sortedEdges.map((edge) => [edge.id, edge.fromId, edge.toId].join("|"));
  return [title.trim(), nodeLines.join("\n"), edgeLines.join("\n")].join("\x1f");
}

function blueprintCanvasFingerprint(title, nodes, edges) {
  return fnv1aHex(canonicalizeBlueprint(title, nodes, edges));
}

const baseNodes = [
  { id: "a", kind: "tool", slug: "onchainai", step: 1 },
  { id: "b", kind: "note", text: "hello" },
];

test("blueprintCanvasFingerprint is deterministic and step-sensitive", () => {
  const fp1 = blueprintCanvasFingerprint("Agent stack", baseNodes, []);
  assert.match(fp1, /^fp:[0-9a-f]{8}$/);
  assert.equal(fp1, blueprintCanvasFingerprint("Agent stack", baseNodes, []));

  const withStep2 = [
    { id: "a", kind: "tool", slug: "onchainai", step: 2 },
    { id: "b", kind: "note", text: "hello" },
  ];
  assert.notEqual(fp1, blueprintCanvasFingerprint("Agent stack", withStep2, []));
});

test("blueprintCanvasFingerprint ignores node order", () => {
  const reversed = [...baseNodes].reverse();
  assert.equal(
    blueprintCanvasFingerprint("T", baseNodes, []),
    blueprintCanvasFingerprint("T", reversed, []),
  );
});