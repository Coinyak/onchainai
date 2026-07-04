"use client";

import { ChainLogo } from "@/components/tools/ChainLogo";
import type { ChainMeta } from "@/lib/chains";

interface BlueprintToolChainMemoProps {
  availableChains: ChainMeta[];
  selectedChainIds: string[];
  nodeSelected: boolean;
  readOnly: boolean;
  onChange: (chainIds: string[]) => void;
}

export function BlueprintToolChainMemo({
  availableChains,
  selectedChainIds,
  nodeSelected,
  readOnly,
  onChange,
}: BlueprintToolChainMemoProps) {
  if (availableChains.length === 0) return null;

  const selectedSet = new Set(selectedChainIds);
  const showPicker = nodeSelected && !readOnly;

  const toggleChain = (chainId: string) => {
    if (selectedSet.has(chainId)) {
      onChange(selectedChainIds.filter((id) => id !== chainId));
      return;
    }
    onChange([...selectedChainIds, chainId]);
  };

  const displayedChains = availableChains.filter((chain) => selectedSet.has(chain.id));

  return (
    <div
      className="blueprint-node-tool-chains"
      data-testid="blueprint-tool-chain-memo"
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {showPicker && (
        <div
          className="blueprint-node-tool-chain-picker"
          role="group"
          aria-label="Select chains for this tool"
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
                onClick={() => toggleChain(chain.id)}
              >
                <ChainLogo id={chain.id} label={chain.label} size={16} decorative />
              </button>
            );
          })}
        </div>
      )}
      <div className="blueprint-node-tool-chain-badges" aria-label="Selected chains">
        {displayedChains.map((chain) => (
          <span
            key={chain.id}
            className="blueprint-node-tool-chain-badge"
            title={chain.label}
          >
            <ChainLogo id={chain.id} label={chain.label} size={14} decorative />
          </span>
        ))}
        {showPicker && displayedChains.length === 0 && (
          <span className="blueprint-node-tool-chain-hint">Chains</span>
        )}
      </div>
    </div>
  );
}