"use client";

import { useMemo, useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import { useQuery } from "@tanstack/react-query";
import { listMyToolkit, searchTools, type PublicTool, type PublicToolSummary } from "@/lib/api";
import { CHAIN_CATALOG, chainTagsForTool, type ChainMeta } from "@/lib/chains";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { ChainLogo } from "@/components/tools/ChainLogo";
import { Badge } from "@/components/ui/Badge";
import { typeBadgeLabel } from "@/lib/format";

interface BlueprintPaletteProps {
  readOnly: boolean;
  onAddTool: (tool: PublicTool | PublicToolSummary) => void;
  onAddChain: (chain: ChainMeta) => void;
}

const PALETTE_CHAIN_VISIBLE = 4;

function PaletteToolItem({
  tool,
  readOnly,
  onAddTool,
}: {
  tool: PublicTool | PublicToolSummary;
  readOnly: boolean;
  onAddTool: (tool: PublicTool | PublicToolSummary) => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `palette-${tool.slug}`,
    data: { type: "palette-tool", tool },
    disabled: readOnly,
  });

  const chains = chainTagsForTool(tool.chains);
  const visibleChains = chains.slice(0, PALETTE_CHAIN_VISIBLE);
  const extraChains = Math.max(0, chains.length - PALETTE_CHAIN_VISIBLE);

  return (
    <button
      ref={setNodeRef}
      type="button"
      className={`blueprint-palette-item${isDragging ? " blueprint-palette-item-dragging" : ""}`}
      onClick={() => !readOnly && onAddTool(tool)}
      {...listeners}
      {...attributes}
    >
      <ToolLogo
        name={tool.name}
        logoUrl={tool.logo_url}
        logoMonogram={tool.logo_monogram}
        status={tool.status}
        size={32}
      />
      <span className="blueprint-palette-item-text">
        <span className="blueprint-palette-item-name">{tool.name}</span>
        <Badge variant="neutral">{typeBadgeLabel(tool.type)}</Badge>
        {visibleChains.length > 0 && (
          <span className="blueprint-palette-item-chains" aria-label="Supported networks">
            {visibleChains.map((chain) => (
              <ChainLogo key={chain.id} id={chain.id} label={chain.label} size={14} decorative />
            ))}
            {extraChains > 0 && (
              <span className="blueprint-palette-item-chains-more" title={`${extraChains} more networks`}>
                +{extraChains}
              </span>
            )}
          </span>
        )}
      </span>
    </button>
  );
}

function PaletteChainItem({
  chain,
  readOnly,
  onAddChain,
}: {
  chain: ChainMeta;
  readOnly: boolean;
  onAddChain: (chain: ChainMeta) => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `palette-chain-${chain.id}`,
    data: { type: "palette-chain", chain },
    disabled: readOnly,
  });

  return (
    <button
      ref={setNodeRef}
      type="button"
      className={`blueprint-palette-item blueprint-palette-item-chain${isDragging ? " blueprint-palette-item-dragging" : ""}`}
      onClick={() => !readOnly && onAddChain(chain)}
      {...listeners}
      {...attributes}
    >
      <ChainLogo id={chain.id} label={chain.label} size={32} decorative />
      <span className="blueprint-palette-item-text">
        <span className="blueprint-palette-item-name">{chain.label}</span>
      </span>
    </button>
  );
}

export function BlueprintPalette({ readOnly, onAddTool, onAddChain }: BlueprintPaletteProps) {
  const [tab, setTab] = useState<"search" | "networks" | "toolkit">("search");
  const [query, setQuery] = useState("");
  const [chainQuery, setChainQuery] = useState("");

  const searchQuery = useQuery({
    queryKey: ["blueprint-search", query],
    queryFn: () => searchTools({ query }),
    enabled: tab === "search" && query.trim().length >= 2,
  });

  const toolkitQuery = useQuery({
    queryKey: ["toolkit"],
    queryFn: listMyToolkit,
    enabled: tab === "toolkit",
  });

  const searchResults = searchQuery.data ?? [];
  const toolkitTools = toolkitQuery.data?.items.map((item) => item.tool) ?? [];
  const chainResults = useMemo(() => {
    const q = chainQuery.trim().toLowerCase();
    const sorted = [...CHAIN_CATALOG].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return a.label.localeCompare(b.label);
    });
    if (!q) return sorted;
    return sorted.filter(
      (chain) =>
        chain.label.toLowerCase().includes(q) ||
        chain.id.toLowerCase().includes(q) ||
        chain.aliases.some((alias) => alias.toLowerCase().includes(q)),
    );
  }, [chainQuery]);

  return (
    <aside className="blueprint-palette" aria-label="Tool palette">
      <div className="blueprint-palette-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={tab === "search"}
          className={`blueprint-palette-tab${tab === "search" ? " blueprint-palette-tab-active" : ""}`}
          onClick={() => setTab("search")}
        >
          Search
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "networks"}
          className={`blueprint-palette-tab${tab === "networks" ? " blueprint-palette-tab-active" : ""}`}
          onClick={() => setTab("networks")}
        >
          Networks
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "toolkit"}
          className={`blueprint-palette-tab${tab === "toolkit" ? " blueprint-palette-tab-active" : ""}`}
          onClick={() => setTab("toolkit")}
        >
          Toolkit
        </button>
      </div>

      {tab === "search" && (
        <div className="blueprint-palette-panel" role="tabpanel">
          <input
            type="search"
            className="blueprint-palette-search"
            placeholder="Search tools..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            disabled={readOnly}
          />
          {query.trim().length < 2 && (
            <p className="blueprint-palette-hint">Type at least 2 characters to search.</p>
          )}
          {searchQuery.isLoading && query.trim().length >= 2 && (
            <p className="blueprint-palette-hint">Searching...</p>
          )}
          {searchQuery.data && searchResults.length === 0 && query.trim().length >= 2 && (
            <p className="blueprint-palette-hint">No tools found.</p>
          )}
          <div className="blueprint-palette-list">
            {searchResults.map((tool) => (
              <PaletteToolItem
                key={tool.slug}
                tool={tool}
                readOnly={readOnly}
                onAddTool={onAddTool}
              />
            ))}
          </div>
        </div>
      )}

      {tab === "networks" && (
        <div className="blueprint-palette-panel" role="tabpanel">
          <input
            type="search"
            className="blueprint-palette-search"
            placeholder="Search networks..."
            value={chainQuery}
            onChange={(e) => setChainQuery(e.target.value)}
            disabled={readOnly}
          />
          <p className="blueprint-palette-hint">
            Drag a network logo sticker onto the canvas.
          </p>
          <div className="blueprint-palette-list">
            {chainResults.map((chain) => (
              <PaletteChainItem
                key={chain.id}
                chain={chain}
                readOnly={readOnly}
                onAddChain={onAddChain}
              />
            ))}
          </div>
        </div>
      )}

      {tab === "toolkit" && (
        <div className="blueprint-palette-panel" role="tabpanel">
          {toolkitQuery.isLoading && <p className="blueprint-palette-hint">Loading toolkit...</p>}
          {toolkitQuery.data && toolkitTools.length === 0 && (
            <p className="blueprint-palette-hint">No saved tools yet. Bookmark tools from the directory.</p>
          )}
          <div className="blueprint-palette-list">
            {toolkitTools.map((tool) => (
              <PaletteToolItem
                key={tool.slug}
                tool={tool}
                readOnly={readOnly}
                onAddTool={onAddTool}
              />
            ))}
          </div>
        </div>
      )}
    </aside>
  );
}