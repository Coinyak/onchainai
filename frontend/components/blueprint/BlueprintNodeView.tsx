"use client";

import { useRef } from "react";
import { useDraggable } from "@dnd-kit/core";
import { ExternalLink, X } from "lucide-react";
import type { BlueprintNode, Tool } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { typeBadgeLabel } from "@/lib/format";

interface BlueprintNodeViewProps {
  node: BlueprintNode;
  tool?: Tool | null;
  toolMissing?: boolean;
  selected: boolean;
  readOnly: boolean;
  isDragging?: boolean;
  onSelect: (id: string) => void;
  onRemove: (id: string) => void;
  onTextChange: (id: string, text: string) => void;
}

export function BlueprintNodeView({
  node,
  tool,
  toolMissing = false,
  selected,
  readOnly,
  isDragging = false,
  onSelect,
  onRemove,
  onTextChange,
}: BlueprintNodeViewProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: node.id,
    data: { type: "canvas-node", nodeId: node.id },
    disabled: readOnly,
  });

  const noteRef = useRef<HTMLTextAreaElement>(null);

  const translate = transform
    ? `translate3d(${transform.x}px, ${transform.y}px, 0)`
    : undefined;

  const className = [
    "blueprint-node",
    node.kind === "note" ? "blueprint-node-note" : "blueprint-node-tool",
    selected ? "blueprint-node-selected" : "",
    isDragging ? "blueprint-node-dragging" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const handleDoubleClick = () => {
    if (node.kind === "tool" && tool && !toolMissing) {
      window.open(`/tools/${tool.slug}`, "_blank", "noopener,noreferrer");
    }
  };

  return (
    <div
      ref={setNodeRef}
      className={className}
      data-testid="blueprint-node"
      data-node-id={node.id}
      style={{
        left: node.x,
        top: node.y,
        transform: translate,
      }}
      onClick={(e) => {
        e.stopPropagation();
        onSelect(node.id);
      }}
      onDoubleClick={handleDoubleClick}
      onFocus={() => onSelect(node.id)}
      {...listeners}
      {...attributes}
      tabIndex={0}
      role="group"
      aria-label={
        node.kind === "tool"
          ? tool?.name ?? node.slug ?? "Tool node"
          : "Note node"
      }
    >
      {node.kind === "tool" ? (
        <>
          {!readOnly && (
            <button
              type="button"
              className="blueprint-node-remove"
              aria-label="Remove node"
              onClick={(e) => {
                e.stopPropagation();
                onRemove(node.id);
              }}
              onPointerDown={(e) => e.stopPropagation()}
            >
              <X size={14} />
            </button>
          )}
          {toolMissing || !tool ? (
            <div className="blueprint-node-ghost">
              <span className="blueprint-node-ghost-label">Removed tool</span>
              <span className="blueprint-node-ghost-slug">{node.slug}</span>
            </div>
          ) : (
            <>
              <ToolLogo
                name={tool.name}
                logoUrl={tool.logo_url}
                logoMonogram={tool.logo_monogram}
                size={32}
              />
              <span className="blueprint-node-tool-text">
                <span className="blueprint-node-tool-name">{tool.name}</span>
                <Badge variant="neutral">{typeBadgeLabel(tool.type)}</Badge>
              </span>
              <a
                href={`/tools/${tool.slug}`}
                target="_blank"
                rel="noopener noreferrer"
                className="blueprint-node-link"
                aria-label={`Open ${tool.name} in new tab`}
                onClick={(e) => e.stopPropagation()}
                onPointerDown={(e) => e.stopPropagation()}
              >
                <ExternalLink size={14} />
              </a>
            </>
          )}
        </>
      ) : (
        <>
          {!readOnly && (
            <button
              type="button"
              className="blueprint-node-remove"
              aria-label="Remove note"
              onClick={(e) => {
                e.stopPropagation();
                onRemove(node.id);
              }}
              onPointerDown={(e) => e.stopPropagation()}
            >
              <X size={14} />
            </button>
          )}
          <textarea
            ref={noteRef}
            className="blueprint-node-note-input"
            placeholder="Add a note..."
            value={node.text ?? ""}
            maxLength={2000}
            readOnly={readOnly}
            onChange={(e) => onTextChange(node.id, e.target.value)}
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          />
        </>
      )}
    </div>
  );
}