const cmpStr = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

function nodeFlowLabel(node) {
  if (node.kind === "tool") return node.slug ?? node.id;
  if (node.kind === "note") {
    const text = node.text?.trim() ?? "";
    if (!text) return "note";
    const chars = [...text];
    if (chars.length > 48) return `note: ${chars.slice(0, 48).join("")}…`;
    return `note: ${text}`;
  }
  if (node.kind === "chain") return `chain: ${node.chainId ?? "unknown"}`;
  return node.id;
}

function walkFlowSegment(
  startIdx,
  flowEdges,
  outEdges,
  inDeg,
  outDeg,
  labelOf,
  visited,
) {
  let line = labelOf(flowEdges[startIdx].from);
  let cur = startIdx;
  for (;;) {
    visited[cur] = true;
    const edge = flowEdges[cur];
    line += edge.label ? ` →(${edge.label}) ` : " → ";
    line += labelOf(edge.to);

    const internal =
      (inDeg.get(edge.to) ?? 0) === 1 && (outDeg.get(edge.to) ?? 0) === 1;
    if (internal) {
      const next = outEdges.get(edge.to)?.[0];
      if (next !== undefined && !visited[next]) {
        cur = next;
        continue;
      }
    }
    break;
  }
  return line;
}

/** Mirrors Rust build_flow_section; shared by blueprint-export.ts and unit tests. */
export function buildFlowSection(nodes, edges) {
  const nodeMap = new Map(nodes.map((node) => [node.id, node]));
  const labelOf = (id) => {
    const node = nodeMap.get(id);
    return node ? nodeFlowLabel(node) : id;
  };

  const flowEdges = [];
  for (const edge of edges) {
    const from = edge.fromId?.trim() ?? "";
    const to = edge.toId?.trim() ?? "";
    if (!from || !to) continue;
    if (!nodeMap.has(from) || !nodeMap.has(to)) continue;
    const label = edge.label?.trim();
    flowEdges.push({ from, to, label: label ? label : undefined });
  }

  if (flowEdges.length === 0) return "(no flow edges defined)";

  const inDeg = new Map();
  const outDeg = new Map();
  const outEdges = new Map();
  for (const node of nodes) {
    inDeg.set(node.id, 0);
    outDeg.set(node.id, 0);
  }
  flowEdges.forEach((edge, idx) => {
    outDeg.set(edge.from, (outDeg.get(edge.from) ?? 0) + 1);
    inDeg.set(edge.to, (inDeg.get(edge.to) ?? 0) + 1);
    if (!inDeg.has(edge.from)) inDeg.set(edge.from, 0);
    if (!outDeg.has(edge.to)) outDeg.set(edge.to, 0);
    const list = outEdges.get(edge.from) ?? [];
    list.push(idx);
    outEdges.set(edge.from, list);
  });

  for (const list of outEdges.values()) {
    list.sort((a, b) => cmpStr(labelOf(flowEdges[a].to), labelOf(flowEdges[b].to)));
  }

  const visited = new Array(flowEdges.length).fill(false);
  const lines = [];

  const junctions = [...outEdges.keys()].sort(
    (a, b) => cmpStr(labelOf(a), labelOf(b)) || cmpStr(a, b),
  );
  for (const from of junctions) {
    const isJunction =
      (inDeg.get(from) ?? 0) !== 1 || (outDeg.get(from) ?? 0) !== 1;
    if (!isJunction) continue;
    for (const idx of outEdges.get(from) ?? []) {
      if (!visited[idx]) {
        lines.push(
          walkFlowSegment(idx, flowEdges, outEdges, inDeg, outDeg, labelOf, visited),
        );
      }
    }
  }

  for (let idx = 0; idx < flowEdges.length; idx += 1) {
    if (!visited[idx]) {
      lines.push(
        walkFlowSegment(idx, flowEdges, outEdges, inDeg, outDeg, labelOf, visited),
      );
    }
  }

  const touched = new Set(flowEdges.flatMap((edge) => [edge.from, edge.to]));
  const orphans = nodes
    .filter((node) => !touched.has(node.id))
    .map(nodeFlowLabel)
    .sort(cmpStr);
  lines.push(...orphans);

  if (lines.length === 0) return "(no flow edges defined)";
  return lines.map((line) => `- ${line}`).join("\n");
}