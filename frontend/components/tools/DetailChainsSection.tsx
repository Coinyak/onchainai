"use client";

import { useState } from "react";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { DetailFilterChip } from "@/components/tools/DetailFilterChip";
import { chainTagsForTool } from "@/lib/chains";
import { singleFilterHref } from "@/lib/browser-query";

const CHAINS_VISIBLE_DEFAULT = 8;

interface DetailChainsSectionProps {
  chains: string[];
}

export function DetailChainsSection({ chains: chainIds }: DetailChainsSectionProps) {
  const [expanded, setExpanded] = useState(false);
  const chains = chainTagsForTool(chainIds);

  if (chains.length === 0) return null;

  const visible = expanded ? chains : chains.slice(0, CHAINS_VISIBLE_DEFAULT);
  const extra = Math.max(0, chains.length - CHAINS_VISIBLE_DEFAULT);

  return (
    <section className="detail-section">
      <h2 className="text-h2 mb-3">Chains</h2>
      <div className="detail-chains">
        {visible.map((c) => (
          <DetailFilterChip
            key={c.id}
            href={singleFilterHref("chain", c.id)}
            label={c.label}
          >
            <ChainLogo id={c.id} label={c.label} size={24} className="detail-chain-logo" decorative />
          </DetailFilterChip>
        ))}
        {!expanded && extra > 0 && (
          <button
            type="button"
            className="detail-chains-more"
            onClick={() => setExpanded(true)}
            aria-label={`Show ${extra} more chains`}
          >
            +{extra} more
          </button>
        )}
      </div>
    </section>
  );
}