"use client";

import { forwardRef } from "react";
import { ExternalLink, Hash, Link2, X } from "lucide-react";
import { BLUEPRINT_NODE_MAX_STEP, parseBlueprintStepInput } from "@/lib/blueprint-utils";

interface BlueprintNodeRailProps {
  nodeKind: "tool" | "chain" | "note";
  visible: boolean;
  readOnly: boolean;
  showChainsButton?: boolean;
  showStepButton?: boolean;
  stepValue?: number;
  toolName?: string;
  onOpenTool?: () => void;
  onOpenChains?: () => void;
  onStepChange?: (step: number | undefined) => void;
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
      stepValue,
      toolName,
      onOpenTool,
      onOpenChains,
      onStepChange,
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
        {showStepButton && onStepChange && (
          <label
            className="blueprint-node-rail-step"
            data-testid="blueprint-node-rail-step"
            title="Order number (duplicates allowed)"
          >
            <Hash size={14} aria-hidden="true" />
            <input
              type="number"
              className="blueprint-node-rail-step-input"
              min={1}
              max={BLUEPRINT_NODE_MAX_STEP}
              value={stepValue ?? ""}
              placeholder="#"
              aria-label="Order number"
              onClick={(e) => e.stopPropagation()}
              onPointerDown={(e) => e.stopPropagation()}
              onChange={(e) => onStepChange(parseBlueprintStepInput(e.target.value))}
            />
          </label>
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