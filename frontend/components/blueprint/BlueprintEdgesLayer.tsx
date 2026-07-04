"use client";

import type { BlueprintEdge, BlueprintNode } from "@/lib/api";
import { buildEdgePath } from "@/lib/blueprint-utils";

interface BlueprintEdgesLayerProps {
  edges: BlueprintEdge[];
  nodes: BlueprintNode[];
  selectedEdgeId: string | null;
  readOnly: boolean;
  onSelectEdge: (id: string) => void;
}

function edgeMidpoint(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): { x: number; y: number } {
  return { x: (x1 + x2) / 2, y: (y1 + y2) / 2 };
}

export function BlueprintEdgesLayer({
  edges,
  nodes,
  selectedEdgeId,
  readOnly,
  onSelectEdge,
}: BlueprintEdgesLayerProps) {
  const nodeById = new Map(nodes.map((node) => [node.id, node]));

  return (
    <svg
      className="blueprint-edges-layer"
      aria-hidden="true"
      data-testid="blueprint-edges"
    >
      <defs>
        {edges.map((edge) => (
          <marker
            key={`marker-${edge.id}`}
            id={`blueprint-arrow-${edge.id}`}
            markerWidth="8"
            markerHeight="8"
            refX="7"
            refY="4"
            orient="auto"
            markerUnits="strokeWidth"
          >
            <path d="M0,0 L8,4 L0,8 Z" fill={edge.color} />
          </marker>
        ))}
      </defs>
      {edges.map((edge) => {
        const from = nodeById.get(edge.fromId);
        const to = nodeById.get(edge.toId);
        if (!from || !to) return null;

        const { x1, y1, x2, y2 } = buildEdgePath(from, to);
        const selected = selectedEdgeId === edge.id;
        const markerEnd =
          edge.style === "arrow" ? `url(#blueprint-arrow-${edge.id})` : undefined;

        return (
          <g key={edge.id}>
            <line
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke="transparent"
              strokeWidth={12}
              className="blueprint-edge-hit"
              onClick={(e) => {
                e.stopPropagation();
                if (!readOnly) onSelectEdge(edge.id);
              }}
            />
            <line
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke={edge.color}
              strokeWidth={selected ? 2.5 : 2}
              markerEnd={markerEnd}
              className={selected ? "blueprint-edge blueprint-edge-selected" : "blueprint-edge"}
            />
            {!readOnly && (
              <circle
                cx={edgeMidpoint(x1, y1, x2, y2).x}
                cy={edgeMidpoint(x1, y1, x2, y2).y}
                r={5}
                className="blueprint-edge-handle"
                onClick={(e) => {
                  e.stopPropagation();
                  onSelectEdge(edge.id);
                }}
              />
            )}
          </g>
        );
      })}
    </svg>
  );
}