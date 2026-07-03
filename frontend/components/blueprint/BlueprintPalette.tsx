"use client";

import { useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import { useQuery } from "@tanstack/react-query";
import { listMyToolkit, searchTools, type Tool } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { typeBadgeLabel } from "@/lib/format";

interface BlueprintPaletteProps {
  readOnly: boolean;
  onAddTool: (tool: Tool) => void;
}

function PaletteToolItem({
  tool,
  readOnly,
  onAddTool,
}: {
  tool: Tool;
  readOnly: boolean;
  onAddTool: (tool: Tool) => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `palette-${tool.slug}`,
    data: { type: "palette-tool", tool },
    disabled: readOnly,
  });

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
        size={32}
      />
      <span className="blueprint-palette-item-text">
        <span className="blueprint-palette-item-name">{tool.name}</span>
        <Badge variant="neutral">{typeBadgeLabel(tool.type)}</Badge>
      </span>
    </button>
  );
}

export function BlueprintPalette({ readOnly, onAddTool }: BlueprintPaletteProps) {
  const [tab, setTab] = useState<"search" | "toolkit">("search");
  const [query, setQuery] = useState("");

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
          aria-selected={tab === "toolkit"}
          className={`blueprint-palette-tab${tab === "toolkit" ? " blueprint-palette-tab-active" : ""}`}
          onClick={() => setTab("toolkit")}
        >
          My Toolkit
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