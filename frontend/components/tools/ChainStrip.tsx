"use client";

import { useState } from "react";
import Link from "next/link";
import {
  type BrowserBase,
  clearAxis,
  toggleMulti,
  parseMulti,
  browserBasePath,
} from "@/lib/browser-query";
import { stripChains, chainFilterActive } from "@/lib/chains";
import { ChainLogo } from "@/components/tools/ChainLogo";

interface ChainStripProps {
  base: BrowserBase;
  queryBase: string;
  activeChain?: string;
  chainCounts: [string, number][];
}

export function ChainStrip({ base, queryBase, activeChain, chainCounts }: ChainStripProps) {
  const [expanded, setExpanded] = useState(false);
  const basePath = browserBasePath(base);
  const chainActive = parseMulti(activeChain);
  const allHref = clearAxis(basePath, queryBase, "chain");
  const allActive = chainActive.length === 0;

  const chains = stripChains(chainCounts);
  const primary = chains.slice(0, 20);
  const overflow = chains.slice(20);
  const overflowCount = overflow.length;

  return (
    <div className="chain-strip" role="group" aria-label="Filter by chain">
      <div className="chain-strip-scroll">
        <Link
          href={allHref}
          className={allActive ? "chain-tile chain-tile-all active" : "chain-tile chain-tile-all"}
          aria-label="All chains"
          title="All chains"
          aria-pressed={allActive}
        >
          All
        </Link>

        {primary.map((entry) => {
          const href = toggleMulti(basePath, queryBase, "chain", entry.id, chainActive);
          const isActive = chainFilterActive(entry, chainActive);
          return (
            <Link
              key={entry.id}
              href={href}
              className={isActive ? "chain-tile chain-tile-logo active" : "chain-tile chain-tile-logo"}
              aria-label={entry.label}
              title={entry.label}
              aria-pressed={isActive}
            >
              <ChainLogo id={entry.id} label={entry.label} size={36} />
            </Link>
          );
        })}

        {expanded &&
          overflow.map((entry) => {
            const href = toggleMulti(basePath, queryBase, "chain", entry.id, chainActive);
            const isActive = chainFilterActive(entry, chainActive);
            return (
              <Link
                key={entry.id}
                href={href}
                className={isActive ? "chain-tile chain-tile-logo active" : "chain-tile chain-tile-logo"}
                aria-label={entry.label}
                title={entry.label}
                aria-pressed={isActive}
              >
                <ChainLogo id={entry.id} label={entry.label} size={36} />
              </Link>
            );
          })}

        {overflowCount > 0 && (
          <button
            type="button"
            className={expanded ? "chain-tile chain-tile-more active" : "chain-tile chain-tile-more"}
            aria-label={expanded ? "Hide extra chains" : `Show ${overflowCount} more chains`}
            title={expanded ? "Hide extra chains" : `Show ${overflowCount} more chains`}
            aria-expanded={expanded}
            onClick={(e) => {
              e.stopPropagation();
              setExpanded((v) => !v);
            }}
          >
            {expanded ? "Less" : `+${overflowCount}`}
          </button>
        )}
      </div>
    </div>
  );
}