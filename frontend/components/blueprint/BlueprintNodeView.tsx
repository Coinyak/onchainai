"use client";

import { useRef, useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import type { BlueprintNode, PublicTool } from "@/lib/api";
import { BlueprintNodeRail } from "@/components/blueprint/BlueprintNodeRail";
import { BlueprintToolChainMemo } from "@/components/blueprint/BlueprintToolChainMemo";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { typeBadgeLabel } from "@/lib/format";
import {
  BLUEPRINT_NODE_TOOL_TYPE_MIN_H,
  clampNodeHeight,
  BLUEPRINT_NODE_MAX_STEP,
  clampNodeWidth,
  getNodeBounds,
  parseBlueprintStepInput,
  toolChainsForNode,
  type BlueprintPortSide,
} from "@/lib/blueprint-utils";

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
  onResize?: (id: string, w: number, h: number) => void;
  onStepChange?: (id: string, step: number | undefined) => void;
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
  onResize,
  onStepChange,
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
  const resizeRef = useRef<{
    startX: number;
    startY: number;
    startW: number;
    startH: number;
  } | null>(null);
  const [hovered, setHovered] = useState(false);
  const [internalChainsOpen, setInternalChainsOpen] = useState(false);

  const railVisible = showRail ?? (selected || hovered);
  const chainsOpen = chainsPopoverOpen ?? internalChainsOpen;
  const availableChains =
    node.kind === "tool" && tool && !toolMissing ? toolChainsForNode(tool.chains) : [];

  const isSizable = node.kind === "tool" || node.kind === "note";
  const bounds = getNodeBounds(node);
  const sizeStyle = isSizable
    ? { width: bounds.w, height: bounds.h }
    : undefined;
  // Collapse optional tool rows as the card shrinks so nothing clips.
  const showTypeTag = bounds.h >= BLUEPRINT_NODE_TOOL_TYPE_MIN_H;
  const canResize = isSizable && !readOnly && (selected || railVisible);
  const showChainsOutside =
    node.kind === "tool" && tool && !toolMissing && availableChains.length > 0;

  const handleResizePointerDown = (e: React.PointerEvent<HTMLSpanElement>) => {
    if (readOnly) return;
    e.stopPropagation();
    e.preventDefault();
    resizeRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      startW: bounds.w,
      startH: bounds.h,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handleResizePointerMove = (e: React.PointerEvent<HTMLSpanElement>) => {
    const start = resizeRef.current;
    if (!start) return;
    e.stopPropagation();
    const w = clampNodeWidth(start.startW + (e.clientX - start.startX));
    const h = clampNodeHeight(start.startH + (e.clientY - start.startY));
    onResize?.(node.id, w, h);
  };

  const handleResizePointerUp = (e: React.PointerEvent<HTMLSpanElement>) => {
    if (!resizeRef.current) return;
    e.stopPropagation();
    resizeRef.current = null;
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // pointer already released
    }
  };

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

  // Ports appear on focus or hover so the canvas stays clean but links stay discoverable.
  const showPorts = !readOnly && (selected || connectPending || hovered);

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

      {!readOnly && selected && onStepChange ? (
        <input
          type="number"
          className="blueprint-node-step blueprint-node-step-input"
          min={1}
          max={BLUEPRINT_NODE_MAX_STEP}
          value={node.step ?? ""}
          placeholder="#"
          aria-label="Order number"
          data-testid="blueprint-node-step-input"
          onClick={(e) => e.stopPropagation()}
          onPointerDown={(e) => e.stopPropagation()}
          onChange={(e) => onStepChange(node.id, parseBlueprintStepInput(e.target.value))}
        />
      ) : node.step != null ? (
        <span className="blueprint-node-step" aria-label={`Step ${node.step}`}>
          {node.step}
        </span>
      ) : null}

      <div
        ref={setNodeRef}
        className={bodyClassName}
        style={sizeStyle}
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
              {showTypeTag && (
                <div className="blueprint-node-tool-row2">
                  <span className="blueprint-node-type-tag">{typeBadgeLabel(tool.type)}</span>
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

      {showChainsOutside ? (
        <div
          className="blueprint-node-tool-chains-outer"
          style={{ width: bounds.w }}
        >
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
      ) : null}

      {canResize && (
        <span
          className="blueprint-node-resize"
          data-testid="blueprint-node-resize"
          role="slider"
          aria-label="Resize card"
          aria-valuenow={bounds.w}
          onPointerDown={handleResizePointerDown}
          onPointerMove={handleResizePointerMove}
          onPointerUp={handleResizePointerUp}
          onClick={(e) => e.stopPropagation()}
        />
      )}

      <BlueprintNodeRail
        ref={chainsButtonRef}
        nodeKind={node.kind}
        visible={railVisible}
        readOnly={readOnly}
        showChainsButton={availableChains.length > 0}
        showStepButton={isSizable || node.kind === "chain"}
        stepValue={node.step}
        toolName={tool?.name}
        onOpenTool={node.kind === "tool" && tool && !toolMissing ? openTool : undefined}
        onOpenChains={availableChains.length > 0 ? handleOpenChains : undefined}
        onStepChange={
          onStepChange
            ? (step) => onStepChange(node.id, step)
            : undefined
        }
        onRemove={() => onRemove(node.id)}
      />
    </div>
  );
}