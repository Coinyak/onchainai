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
import {
  stripChains,
  chainFilterActive,
  hasChainLogo,
  STRIP_PRIMARY_VISIBLE,
} from "@/lib/chains";
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
  const withLogo = chains.filter((entry) => hasChainLogo(entry.id));
  const withoutLogo = chains.filter((entry) => !hasChainLogo(entry.id));
  const tileChains = withLogo.slice(0, STRIP_PRIMARY_VISIBLE);
  const overflowChains = [...withLogo.slice(STRIP_PRIMARY_VISIBLE), ...withoutLogo];
  const overflowCount = overflowChains.length;

  return (
    <div className="chain-strip" role="group" aria-label="Filter by chain">
      <div className="chain-strip-viewport">
        <div className="chain-strip-scroll" tabIndex={0}>
          <Link
            href={allHref}
            scroll={false}
            className={allActive ? "chain-tile chain-tile-all active" : "chain-tile chain-tile-all"}
            aria-label="All chains"
            title="All chains"
            aria-current={allActive ? "page" : undefined}
          >
            All
          </Link>

          {tileChains.map((entry) => {
            const href = toggleMulti(basePath, queryBase, "chain", entry.id, chainActive);
            const isActive = chainFilterActive(entry, chainActive);
            return (
              <Link
                key={entry.id}
                href={href}
                scroll={false}
                className={isActive ? "chain-tile chain-tile-logo active" : "chain-tile chain-tile-logo"}
                aria-label={entry.label}
                title={entry.label}
                aria-current={isActive ? "page" : undefined}
              >
                <ChainLogo id={entry.id} label={entry.label} size={36} />
              </Link>
            );
          })}
        </div>

        {overflowCount > 0 && (
          <div className="chain-strip-more-anchor">
            <button
              type="button"
              className={expanded ? "chain-tile chain-tile-more active" : "chain-tile chain-tile-more"}
              data-testid="chain-strip-more"
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
          </div>
        )}

        {overflowCount > 0 && <div className="chain-strip-fade" aria-hidden="true" />}
      </div>

      {expanded && overflowCount > 0 && (
        <ul className="chain-strip-overflow-list" role="list">
          {overflowChains.map((entry) => {
            const href = toggleMulti(basePath, queryBase, "chain", entry.id, chainActive);
            const isActive = chainFilterActive(entry, chainActive);
            return (
              <li key={entry.id}>
                <Link
                  href={href}
                  scroll={false}
                  className={isActive ? "chain-overflow-item active" : "chain-overflow-item"}
                  aria-current={isActive ? "page" : undefined}
                >
                  {hasChainLogo(entry.id) ? (
                    <ChainLogo id={entry.id} label={entry.label} size={24} decorative />
                  ) : (
                    <span className="chain-overflow-fallback" aria-hidden="true" />
                  )}
                  <span className="chain-overflow-label">{entry.label}</span>
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}