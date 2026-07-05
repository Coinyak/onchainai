"use client";

import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { searchTools, type PublicToolSummary } from "@/lib/api";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { typeBadgeLabel } from "@/lib/format";

interface CompareAddToolTypeaheadProps {
  selectedSlugs: string[];
  disabled?: boolean;
  onSelect: (slug: string) => void;
}

export function CompareAddToolTypeahead({
  selectedSlugs,
  disabled = false,
  onSelect,
}: CompareAddToolTypeaheadProps) {
  const listboxId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedQuery(query.trim()), 200);
    return () => window.clearTimeout(timer);
  }, [query]);

  const searchQuery = useQuery({
    queryKey: ["compare-add-tool", debouncedQuery],
    queryFn: () => searchTools({ query: debouncedQuery }),
    enabled: !disabled && debouncedQuery.length >= 2,
  });

  const results = useMemo(
    () => (searchQuery.data ?? []).filter((tool) => !selectedSlugs.includes(tool.slug)),
    [searchQuery.data, selectedSlugs],
  );

  const resolvedActiveIndex = useMemo(() => {
    if (!open || results.length === 0) return -1;
    if (activeIndex < 0 || activeIndex >= results.length) return 0;
    return activeIndex;
  }, [activeIndex, open, results.length]);

  useEffect(() => {
    function onPointerDown(event: MouseEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, []);

  function choose(tool: PublicToolSummary) {
    onSelect(tool.slug);
    setQuery("");
    setDebouncedQuery("");
    setOpen(false);
    setActiveIndex(-1);
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) setOpen(true);
      if (results.length === 0) return;
      setActiveIndex((index) => (index + 1) % results.length);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (results.length === 0) return;
      setActiveIndex((index) => (index <= 0 ? results.length - 1 : index - 1));
      return;
    }
    if (event.key === "Enter" && resolvedActiveIndex >= 0 && results[resolvedActiveIndex]) {
      event.preventDefault();
      choose(results[resolvedActiveIndex]);
      return;
    }
    if (event.key === "Escape") {
      setOpen(false);
      setActiveIndex(-1);
    }
  }

  return (
    <div ref={rootRef} className="compare-add-tool" data-testid="compare-add-tool">
      <label className="compare-add-tool-label" htmlFor={`${listboxId}-input`}>
        <Plus size={16} aria-hidden />
        Add tool
      </label>
      <input
        id={`${listboxId}-input`}
        type="search"
        className="compare-add-tool-input"
        placeholder="Search by name..."
        value={query}
        disabled={disabled}
        autoComplete="off"
        role="combobox"
        aria-expanded={open}
        aria-controls={listboxId}
        aria-activedescendant={
          resolvedActiveIndex >= 0 ? `${listboxId}-option-${resolvedActiveIndex}` : undefined
        }
        onFocus={() => setOpen(true)}
        onChange={(event) => {
          setQuery(event.target.value);
          setOpen(true);
        }}
        onKeyDown={onKeyDown}
      />
      {open && debouncedQuery.length >= 2 && (
        <ul id={listboxId} className="compare-add-tool-list" role="listbox">
          {searchQuery.isLoading && (
            <li className="compare-add-tool-hint" role="presentation">
              Searching...
            </li>
          )}
          {!searchQuery.isLoading && results.length === 0 && (
            <li className="compare-add-tool-hint" role="presentation">
              {searchQuery.isError ? "Search failed. Try again." : "No tools found."}
            </li>
          )}
          {results.map((tool, index) => (
            <li key={tool.slug} role="presentation">
              <button
                id={`${listboxId}-option-${index}`}
                type="button"
                role="option"
                aria-selected={index === resolvedActiveIndex}
                className={`compare-add-tool-option${
                  index === resolvedActiveIndex ? " compare-add-tool-option-active" : ""
                }`}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => choose(tool)}
              >
                <ToolLogo
                  name={tool.name}
                  logoUrl={tool.logo_url}
                  logoMonogram={tool.logo_monogram}
                  size={28}
                />
                <span className="compare-add-tool-option-text">
                  <span className="compare-add-tool-option-name">{tool.name}</span>
                  <span className="compare-add-tool-option-type">
                    {typeBadgeLabel(tool.type)}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}