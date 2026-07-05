"use client";

import { forwardRef } from "react";
import { ExternalLink, Hash, Link2, X } from "lucide-react";

interface BlueprintNodeRailProps {
  nodeKind: "tool" | "chain" | "note";
  visible: boolean;
  readOnly: boolean;
  showChainsButton?: boolean;
  showStepButton?: boolean;
  hasStep?: boolean;
  toolName?: string;
  onOpenTool?: () => void;
  onOpenChains?: () => void;
  onToggleStep?: () => void;
  onRemove: () => void;
}

export const BlueprintNodeRail = forwardRef<HTMLButtonElement, BlueprintNodeRailProps>(
  function BlueprintNodeRail(
    {
      nodeKind,
      visible,
      readOnly,
      showChainsButton = false,
      showStepButton = false,
      hasStep = false,
      toolName,
      onOpenTool,
      onOpenChains,
      onToggleStep,
      onRemove,
    },
    chainsButtonRef,
  ) {
    if (readOnly) return null;

    return (
      <div
        className={`blueprint-node-rail${visible ? " blueprint-node-rail-visible" : ""}`}
        aria-hidden={!visible}
        onClick={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
      >
        {nodeKind === "tool" && onOpenTool && (
          <button
            type="button"
            className="blueprint-node-rail-btn"
            aria-label={toolName ? `Open ${toolName} in new tab` : "Open tool in new tab"}
            data-testid="blueprint-node-rail-open"
            onClick={(e) => {
              e.stopPropagation();
              onOpenTool();
            }}
          >
            <ExternalLink size={16} />
          </button>
        )}
        {nodeKind === "tool" && showChainsButton && onOpenChains && (
          <button
            ref={chainsButtonRef}
            type="button"
            className="blueprint-node-rail-btn"
            aria-label="Select chains for this tool"
            data-testid="blueprint-node-rail-chains"
            onClick={(e) => {
              e.stopPropagation();
              onOpenChains();
            }}
          >
            <Link2 size={16} />
          </button>
        )}
        {showStepButton && onToggleStep && (
          <button
            type="button"
            className={`blueprint-node-rail-btn${hasStep ? " blueprint-node-rail-btn-active" : ""}`}
            aria-label={hasStep ? "Remove order number" : "Add order number"}
            aria-pressed={hasStep}
            data-testid="blueprint-node-rail-step"
            onClick={(e) => {
              e.stopPropagation();
              onToggleStep();
            }}
          >
            <Hash size={16} />
          </button>
        )}
        <button
          type="button"
          className="blueprint-node-rail-btn blueprint-node-rail-btn-danger"
          aria-label={
            nodeKind === "chain"
              ? "Remove network sticker"
              : nodeKind === "note"
                ? "Remove note"
                : "Remove node"
          }
          data-testid="blueprint-node-rail-delete"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
        >
          <X size={16} />
        </button>
      </div>
    );
  },
);