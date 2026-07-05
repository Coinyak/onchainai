"use client";

import { useCallback, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { ChainLogo } from "@/components/tools/ChainLogo";
import type { ChainMeta } from "@/lib/chains";
import {
  BLUEPRINT_MAX_TOOL_CHAINS,
  selectedChainsMeta,
} from "@/lib/blueprint-utils";

const MAX_VISIBLE_CHAIN_BADGES = 8;

interface BlueprintToolChainMemoProps {
  availableChains: ChainMeta[];
  selectedChainIds: string[];
  chainsPopoverOpen: boolean;
  readOnly: boolean;
  anchorRef: React.RefObject<HTMLElement | null>;
  onChange: (chainIds: string[]) => void;
  onClose?: () => void;
}

function positionPopover(
  popoverEl: HTMLDivElement | null,
  anchorEl: HTMLElement | null,
) {
  if (!popoverEl || !anchorEl) return;
  const anchorRect = anchorEl.getBoundingClientRect();
  const popoverRect = popoverEl.getBoundingClientRect();
  const margin = 8;
  let left = anchorRect.left - popoverRect.width - margin;
  if (left < margin) {
    left = anchorRect.right + margin;
  }
  popoverEl.style.top = `${anchorRect.top}px`;
  popoverEl.style.left = `${left}px`;
}

export function BlueprintToolChainMemo({
  availableChains,
  selectedChainIds,
  chainsPopoverOpen,
  readOnly,
  anchorRef,
  onChange,
  onClose,
}: BlueprintToolChainMemoProps) {
  const popoverRef = useRef<HTMLDivElement>(null);

  const selectedSet = new Set(selectedChainIds);
  const hasMemoSelection = selectedChainIds.length > 0;
  const memoChains = selectedChainsMeta(selectedChainIds);
  const overflowCount = Math.max(0, memoChains.length - MAX_VISIBLE_CHAIN_BADGES);
  const visibleBadges = overflowCount > 0
    ? memoChains.slice(0, MAX_VISIBLE_CHAIN_BADGES)
    : memoChains;

  const toggleChain = (chainId: string) => {
    if (selectedSet.has(chainId)) {
      onChange(selectedChainIds.filter((id) => id !== chainId));
      return;
    }
    if (selectedChainIds.length >= BLUEPRINT_MAX_TOOL_CHAINS) return;
    onChange([...selectedChainIds, chainId]);
  };

  const syncPopoverPosition = useCallback(() => {
    positionPopover(popoverRef.current, anchorRef.current);
  }, [anchorRef]);

  const setPopoverRef = useCallback(
    (el: HTMLDivElement | null) => {
      popoverRef.current = el;
      if (el) syncPopoverPosition();
    },
    [syncPopoverPosition],
  );

  useEffect(() => {
    if (!chainsPopoverOpen || readOnly) return;

    syncPopoverPosition();
    window.addEventListener("resize", syncPopoverPosition);
    window.addEventListener("scroll", syncPopoverPosition, true);
    return () => {
      window.removeEventListener("resize", syncPopoverPosition);
      window.removeEventListener("scroll", syncPopoverPosition, true);
    };
  }, [chainsPopoverOpen, readOnly, syncPopoverPosition]);

  useEffect(() => {
    if (!chainsPopoverOpen || readOnly) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (popoverRef.current?.contains(target)) return;
      if (anchorRef.current?.contains(target)) return;
      onClose?.();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose?.();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [anchorRef, chainsPopoverOpen, onClose, readOnly]);

  if (availableChains.length === 0) return null;

  return (
    <>
      {hasMemoSelection ? (
        <div
          className="blueprint-node-tool-chains"
          data-testid="blueprint-tool-chain-memo"
          onClick={(e) => e.stopPropagation()}
          onPointerDown={(e) => e.stopPropagation()}
        >
          <div className="blueprint-node-tool-chain-badges" aria-label="Selected networks">
            {visibleBadges.map((chain) => (
              <span
                key={chain.id}
                className="blueprint-node-tool-chain-badge"
                title={chain.label}
              >
                <ChainLogo id={chain.id} label={chain.label} size={14} decorative />
              </span>
            ))}
            {overflowCount > 0 && (
              <span className="blueprint-node-tool-chain-overflow" title={`${overflowCount} more chains`}>
                +{overflowCount}
              </span>
            )}
          </div>
        </div>
      ) : null}

      {chainsPopoverOpen &&
        !readOnly &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            ref={setPopoverRef}
            className="blueprint-node-tool-chain-popover"
            role="group"
            aria-label="Select chains for this tool"
            data-testid="blueprint-tool-chain-popover"
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => e.stopPropagation()}
          >
            {availableChains.map((chain) => {
              const active = selectedSet.has(chain.id);
              return (
                <button
                  key={chain.id}
                  type="button"
                  className={`blueprint-node-tool-chain-option${active ? " blueprint-node-tool-chain-option-active" : ""}`}
                  aria-label={`${active ? "Remove" : "Add"} ${chain.label}`}
                  aria-pressed={active}
                  title={chain.label}
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleChain(chain.id);
                  }}
                >
                  <ChainLogo id={chain.id} label={chain.label} size={16} decorative />
                </button>
              );
            })}
          </div>,
          document.body,
        )}
    </>
  );
}