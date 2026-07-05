"use client";

import { useRef, useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import type { BlueprintNode, PublicTool } from "@/lib/api";
import { BlueprintNodeRail } from "@/components/blueprint/BlueprintNodeRail";
import { BlueprintToolChainMemo } from "@/components/blueprint/BlueprintToolChainMemo";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { typeBadgeLabel } from "@/lib/format";
import { toolChainsForNode, type BlueprintPortSide } from "@/lib/blueprint-utils";

const NODE_PORTS: {
  side: BlueprintPortSide;
  className: string;
  testId: string;
  label: string;
}[] = [
  { side: "top", className: "blueprint-node-port-top", testId: "blueprint-node-port-top", label: "Connect from top" },
  { side: "right", className: "blueprint-node-port-right", testId: "blueprint-node-port-right", label: "Connect from right" },
  { side: "bottom", className: "blueprint-node-port-bottom", testId: "blueprint-node-port-bottom", label: "Connect from bottom" },
  { side: "left", className: "blueprint-node-port-left", testId: "blueprint-node-port-left", label: "Connect from left" },
];

interface BlueprintNodeViewProps {
  node: BlueprintNode;
  tool?: PublicTool | null;
  toolMissing?: boolean;
  chainLabel?: string;
  selected: boolean;
  connectPending?: boolean;
  readOnly: boolean;
  isDragging?: boolean;
  showRail?: boolean;
  chainsPopoverOpen?: boolean;
  onSelect: (id: string) => void;
  onRemove: (id: string) => void;
  onTextChange: (id: string, text: string) => void;
  onChainsChange: (id: string, chains: string[]) => void;
  onOpenChains?: (id: string) => void;
  onCloseChains?: (id: string) => void;
  onToggleChains?: (id: string) => void;
  onPortPointerDown?: (
    nodeId: string,
    side: BlueprintPortSide,
    event: React.PointerEvent<HTMLButtonElement>,
  ) => void;
}

export function BlueprintNodeView({
  node,
  tool,
  toolMissing = false,
  chainLabel,
  selected,
  connectPending = false,
  readOnly,
  isDragging = false,
  showRail,
  chainsPopoverOpen,
  onSelect,
  onRemove,
  onTextChange,
  onChainsChange,
  onOpenChains,
  onCloseChains,
  onToggleChains,
  onPortPointerDown,
}: BlueprintNodeViewProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: node.id,
    data: { type: "canvas-node", nodeId: node.id },
    disabled: readOnly,
  });

  const chainsButtonRef = useRef<HTMLButtonElement>(null);
  const [hovered, setHovered] = useState(false);
  const [internalChainsOpen, setInternalChainsOpen] = useState(false);

  const railVisible = showRail ?? (selected || hovered);
  const chainsOpen = chainsPopoverOpen ?? internalChainsOpen;
  const availableChains =
    node.kind === "tool" && tool && !toolMissing ? toolChainsForNode(tool.chains) : [];

  const translate = transform
    ? `translate3d(${transform.x}px, ${transform.y}px, 0)`
    : undefined;

  const wrapClassName = [
    "blueprint-node-wrap",
    selected ? "is-selected" : "",
    connectPending ? "is-connect-pending" : "",
    isDragging ? "is-dragging" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const bodyClassName = [
    "blueprint-node",
    node.kind === "note"
      ? "blueprint-node-note"
      : node.kind === "chain"
        ? "blueprint-node-chain"
        : "blueprint-node-tool",
  ]
    .filter(Boolean)
    .join(" ");

  const openTool = () => {
    if (node.kind === "tool" && tool && !toolMissing) {
      window.open(`/tools/${tool.slug}`, "_blank", "noopener,noreferrer");
    }
  };

  const handleOpenChains = () => {
    if (onToggleChains) {
      onToggleChains(node.id);
      return;
    }
    onOpenChains?.(node.id);
    if (chainsPopoverOpen === undefined) {
      setInternalChainsOpen(true);
    }
  };

  const handleCloseChains = () => {
    if (onToggleChains) {
      if (chainsOpen) onToggleChains(node.id);
      return;
    }
    onCloseChains?.(node.id);
    if (chainsPopoverOpen === undefined) {
      setInternalChainsOpen(false);
    }
  };

  const showPorts = !readOnly;

  return (
    <div
      className={wrapClassName}
      data-testid="blueprint-node"
      data-node-id={node.id}
      style={{
        left: node.x,
        top: node.y,
        transform: translate,
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={(e) => {
        e.stopPropagation();
        onSelect(node.id);
      }}
      onFocus={() => onSelect(node.id)}
      tabIndex={-1}
    >
      {showPorts &&
        NODE_PORTS.map((port) => (
          <button
            key={port.side}
            type="button"
            className={`blueprint-node-port ${port.className}`}
            data-testid={port.testId}
            data-port={port.side}
            data-node-id={node.id}
            aria-label={port.label}
            onPointerDown={(e) => {
              e.stopPropagation();
              onPortPointerDown?.(node.id, port.side, e);
            }}
            onClick={(e) => e.stopPropagation()}
          />
        ))}

      <div
        ref={setNodeRef}
        className={bodyClassName}
        onDoubleClick={() => openTool()}
        {...listeners}
        {...attributes}
        tabIndex={0}
        role="group"
        aria-label={
          node.kind === "tool"
            ? tool?.name ?? node.slug ?? "Tool node"
            : node.kind === "chain"
              ? chainLabel ?? node.chainId ?? "Network sticker"
              : "Note node"
        }
      >
        {node.kind === "chain" ? (
          <>
            <div className="blueprint-node-chain-circle">
              <ChainLogo
                id={node.chainId ?? ""}
                label={chainLabel ?? node.chainId ?? "Network"}
                size={32}
                decorative
              />
            </div>
            <span className="blueprint-node-chain-label">
              {chainLabel ?? node.chainId ?? "Network"}
            </span>
          </>
        ) : node.kind === "tool" ? (
          toolMissing || !tool ? (
            <div className="blueprint-node-ghost">
              <span className="blueprint-node-ghost-label">Removed tool</span>
              <span className="blueprint-node-ghost-slug">{node.slug}</span>
            </div>
          ) : (
            <>
              <div className="blueprint-node-tool-row1">
                <ToolLogo
                  name={tool.name}
                  logoUrl={tool.logo_url}
                  logoMonogram={tool.logo_monogram}
                  status={tool.status}
                  size={36}
                />
                <span className="blueprint-node-tool-text">
                  <span className="blueprint-node-tool-name">{tool.name}</span>
                </span>
              </div>
              <div className="blueprint-node-tool-row2">
                <span className="blueprint-node-type-tag">{typeBadgeLabel(tool.type)}</span>
              </div>
              {availableChains.length > 0 && (
                <div className="blueprint-node-tool-row3">
                  <BlueprintToolChainMemo
                    availableChains={availableChains}
                    selectedChainIds={node.chains ?? []}
                    chainsPopoverOpen={chainsOpen}
                    readOnly={readOnly}
                    anchorRef={chainsButtonRef}
                    onChange={(chains) => onChainsChange(node.id, chains)}
                    onClose={handleCloseChains}
                  />
                </div>
              )}
            </>
          )
        ) : (
          <textarea
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
        )}
      </div>

      <BlueprintNodeRail
        ref={chainsButtonRef}
        nodeKind={node.kind}
        visible={railVisible}
        readOnly={readOnly}
        showChainsButton={availableChains.length > 0}
        toolName={tool?.name}
        onOpenTool={node.kind === "tool" && tool && !toolMissing ? openTool : undefined}
        onOpenChains={availableChains.length > 0 ? handleOpenChains : undefined}
        onRemove={() => onRemove(node.id)}
      />
    </div>
  );
}