"use client";

import { Fragment } from "react";
import { X } from "lucide-react";
import type { BlueprintEdge, BlueprintNode } from "@/lib/api";
import { buildEdgePath, type BlueprintEndpoint } from "@/lib/blueprint-utils";

interface BlueprintEdgesLayerProps {
  edges: BlueprintEdge[];
  nodes: BlueprintNode[];
  selectedEdgeId: string | null;
  readOnly: boolean;
  onSelectEdge: (id: string) => void;
  onDeleteEdge: (id: string) => void;
  onEndpointPointerDown: (
    edgeId: string,
    end: BlueprintEndpoint,
    event: React.PointerEvent<HTMLSpanElement>,
  ) => void;
}

interface RenderedEdge {
  edge: BlueprintEdge;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  midX: number;
  midY: number;
  selected: boolean;
}

export function BlueprintEdgesLayer({
  edges,
  nodes,
  selectedEdgeId,
  readOnly,
  onSelectEdge,
  onDeleteEdge,
  onEndpointPointerDown,
}: BlueprintEdgesLayerProps) {
  const nodeById = new Map(nodes.map((node) => [node.id, node]));

  const rendered: RenderedEdge[] = [];
  for (const edge of edges) {
    const from = nodeById.get(edge.fromId);
    const to = nodeById.get(edge.toId);
    if (!from || !to) continue;
    const { x1, y1, x2, y2 } = buildEdgePath(from, to);
    rendered.push({
      edge,
      x1,
      y1,
      x2,
      y2,
      midX: (x1 + x2) / 2,
      midY: (y1 + y2) / 2,
      selected: selectedEdgeId === edge.id,
    });
  }

  return (
    <>
      <svg
        className="blueprint-edges-layer"
        aria-hidden="true"
        data-testid="blueprint-edges"
      >
        <defs>
          {rendered
            .filter(({ edge }) => edge.style === "arrow")
            .map(({ edge }) => (
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
        {rendered.map(({ edge, x1, y1, x2, y2, selected }) => {
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
                strokeDasharray={edge.dashed ? "7 5" : undefined}
                markerEnd={markerEnd}
                className={
                  selected ? "blueprint-edge blueprint-edge-selected" : "blueprint-edge"
                }
              />
            </g>
          );
        })}
      </svg>

      <div
        className="blueprint-edge-chrome-layer"
        data-testid="blueprint-edge-chrome"
      >
        {rendered.map(({ edge, x1, y1, x2, y2, midX, midY, selected }) => (
          <Fragment key={edge.id}>
            {edge.label && !selected &&
              (readOnly ? (
                <span
                  className="blueprint-edge-label blueprint-edge-label-readonly"
                  style={{ left: midX, top: midY }}
                  title={edge.label}
                >
                  {edge.label}
                </span>
              ) : (
                <button
                  type="button"
                  className="blueprint-edge-label"
                  style={{ left: midX, top: midY }}
                  title={edge.label}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelectEdge(edge.id);
                  }}
                >
                  {edge.label}
                </button>
              ))}
            {!readOnly && selected && (
              <>
                <button
                  type="button"
                  className="blueprint-edge-delete"
                  data-testid="blueprint-edge-delete"
                  style={{ left: midX, top: midY }}
                  aria-label="Delete link"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDeleteEdge(edge.id);
                  }}
                >
                  <X size={13} aria-hidden="true" />
                </button>
                <span
                  className="blueprint-edge-endpoint"
                  data-testid="blueprint-edge-endpoint-from"
                  style={{ left: x1, top: y1 }}
                  onPointerDown={(e) => onEndpointPointerDown(edge.id, "from", e)}
                  onClick={(e) => e.stopPropagation()}
                />
                <span
                  className="blueprint-edge-endpoint"
                  data-testid="blueprint-edge-endpoint-to"
                  style={{ left: x2, top: y2 }}
                  onPointerDown={(e) => onEndpointPointerDown(edge.id, "to", e)}
                  onClick={(e) => e.stopPropagation()}
                />
              </>
            )}
          </Fragment>
        ))}
      </div>
    </>
  );
}
