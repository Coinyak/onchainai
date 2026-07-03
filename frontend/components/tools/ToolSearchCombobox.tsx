"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { X } from "lucide-react";
import {
  useToolSearchTypeahead,
  optionId,
  TOOL_SEARCH_DEBOUNCE_MS,
  TOOL_SEARCH_PAGE_SIZE,
  type UseToolSearchTypeaheadOptions,
} from "@/hooks/useToolSearchTypeahead";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";
import { typeBadgeLabel } from "@/lib/format";
import type { Tool } from "@/lib/api";

export type ToolSearchComboboxVariant = "hero" | "toolbar";

export interface ToolSearchComboboxProps {
  variant?: ToolSearchComboboxVariant;
  defaultValue?: string;
  placeholder?: string;
  inputClassName?: string;
  /** Called when the user submits a full search (Enter without selection). */
  onSubmitSearch?: (query: string) => void;
  /** Called when the user picks a typeahead result. */
  onSelectTool?: (tool: Tool) => void;
  /** Debounced query changes — use for URL sync in toolbar mode. */
  onDebouncedQueryChange?: (query: string) => void;
  functionFilter?: string;
  chainFilter?: string;
  "data-testid"?: string;
}

const MOBILE_FULLSCREEN_MAX_WIDTH = 767;

function useIsMobileViewport(): boolean {
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia(`(max-width: ${MOBILE_FULLSCREEN_MAX_WIDTH}px)`);
    const update = () => setIsMobile(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  return isMobile;
}

function TypeaheadList({
  listboxId,
  results,
  activeIndex,
  isLoading,
  query,
  onHover,
  onSelect,
}: {
  listboxId: string;
  results: Tool[];
  activeIndex: number;
  isLoading: boolean;
  query: string;
  onHover: (index: number) => void;
  onSelect: (tool: Tool) => void;
}) {
  if (!query.trim()) return null;

  return (
    <ul
      id={listboxId}
      role="listbox"
      className="search-typeahead-list"
      data-testid="search-typeahead-list"
    >
      {isLoading && results.length === 0 && (
        <li className="search-typeahead-status" role="presentation">
          Searching...
        </li>
      )}
      {!isLoading && results.length === 0 && (
        <li className="search-typeahead-status" role="presentation">
          No matching tools
        </li>
      )}
      {results.map((tool, index) => (
        <li
          key={tool.slug}
          id={optionId(listboxId, tool.slug)}
          role="option"
          aria-selected={index === activeIndex}
          className={`search-typeahead-option${index === activeIndex ? " search-typeahead-option-active" : ""}`}
          data-testid={`search-typeahead-option-${tool.slug}`}
          onMouseEnter={() => onHover(index)}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => onSelect(tool)}
        >
          <ToolLogo
            name={tool.name}
            logoUrl={tool.logo_url}
            logoMonogram={tool.logo_monogram}
            size={32}
          />
          <span className="search-typeahead-option-text">
            <span className="search-typeahead-option-name">{tool.name}</span>
            <Badge variant="neutral">{typeBadgeLabel(tool.type)}</Badge>
          </span>
        </li>
      ))}
    </ul>
  );
}

/**
 * Reusable tool search combobox — hero, toolbar, and future ⌘K palette (N2).
 */
export function ToolSearchCombobox({
  variant = "hero",
  defaultValue = "",
  placeholder,
  inputClassName,
  onSubmitSearch,
  onSelectTool,
  onDebouncedQueryChange,
  functionFilter,
  chainFilter,
  "data-testid": dataTestId = "tool-search-combobox",
}: ToolSearchComboboxProps) {
  const router = useRouter();
  const pathname = usePathname();
  const isMobile = useIsMobileViewport();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState(defaultValue);
  const [mobileExpanded, setMobileExpanded] = useState(false);
  const syncingFromProps = useRef(false);
  const lastUrlQuery = useRef(defaultValue.trim());

  const typeaheadOptions: UseToolSearchTypeaheadOptions = {
    query,
    functionFilter,
    chainFilter,
    debounceMs: TOOL_SEARCH_DEBOUNCE_MS,
    pageSize: TOOL_SEARCH_PAGE_SIZE,
  };

  const typeahead = useToolSearchTypeahead(typeaheadOptions);

  useEffect(() => {
    const next = (defaultValue ?? "").trim();
    if (next === lastUrlQuery.current) return;
    lastUrlQuery.current = next;
    syncingFromProps.current = true;
    setQuery(defaultValue ?? "");
    syncingFromProps.current = false;
  }, [defaultValue]);

  useEffect(() => {
    if (syncingFromProps.current || !onDebouncedQueryChange) return;
    const trimmed = typeahead.debouncedQuery;
    if (trimmed === lastUrlQuery.current) return;
    lastUrlQuery.current = trimmed;
    onDebouncedQueryChange(trimmed);
  }, [onDebouncedQueryChange, typeahead.debouncedQuery]);

  useEffect(() => {
    if (!mobileExpanded) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, [mobileExpanded]);

  const defaultPlaceholder =
    variant === "hero"
      ? "Search: asset tracking, trading, DeFi, chain name..."
      : "Search tools...";

  const inputClass =
    inputClassName ??
    (variant === "hero"
      ? "search-input w-full h-12 px-4 text-body-md md:text-mobile-body rounded-md border border-border bg-neutral-bg text-primary outline-none focus:border-tertiary"
      : "toolbar-search-input");

  const navigateToTool = useCallback(
    (tool: Tool) => {
      typeahead.closeDropdown();
      setMobileExpanded(false);
      if (onSelectTool) {
        onSelectTool(tool);
        return;
      }
      router.push(`/tools/${tool.slug}`);
    },
    [onSelectTool, router, typeahead],
  );

  const submitSearch = useCallback(
    (raw: string) => {
      const trimmed = raw.trim();
      if (!trimmed) return;
      typeahead.closeDropdown();
      setMobileExpanded(false);
      lastUrlQuery.current = trimmed;
      if (onSubmitSearch) {
        onSubmitSearch(trimmed);
        return;
      }
      router.push(`${pathname}?q=${encodeURIComponent(trimmed)}`, { scroll: false });
    },
    [onSubmitSearch, pathname, router, typeahead],
  );

  const handleInputChange = (value: string) => {
    setQuery(value);
    if (value.trim()) {
      typeahead.openDropdown();
    } else {
      typeahead.closeDropdown();
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    typeahead.handleKeyDown(event);
    if (event.defaultPrevented) return;

    if (event.key === "Enter") {
      event.preventDefault();
      const selected = typeahead.selectActive();
      if (selected) {
        navigateToTool(selected);
        return;
      }
      submitSearch(query);
    }
  };

  const handleFocus = () => {
    if (isMobile) setMobileExpanded(true);
    if (query.trim() && typeahead.results.length > 0) typeahead.openDropdown();
  };

  const handleBlur = () => {
    if (isMobile && mobileExpanded) return;
    window.setTimeout(() => typeahead.closeDropdown(), 150);
  };

  const closeMobileOverlay = () => {
    setMobileExpanded(false);
    typeahead.closeDropdown();
    inputRef.current?.blur();
  };

  const showDropdown =
    typeahead.isOpen &&
    query.trim().length > 0 &&
    (typeahead.isLoading || typeahead.results.length > 0 || typeahead.debouncedQuery.length > 0);

  const inputElement = (
    <input
      ref={inputRef}
      type="search"
      role="combobox"
      aria-expanded={showDropdown}
      aria-controls={showDropdown ? typeahead.listboxId : undefined}
      aria-activedescendant={typeahead.activeDescendantId}
      aria-autocomplete="list"
      placeholder={placeholder ?? defaultPlaceholder}
      className={inputClass}
      autoComplete="off"
      value={query}
      data-testid={dataTestId}
      onChange={(event) => handleInputChange(event.target.value)}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
    />
  );

  if (isMobile && mobileExpanded) {
    return (
      <div
        className="search-typeahead-mobile-overlay"
        data-testid="search-typeahead-mobile"
        role="dialog"
        aria-label="Search tools"
      >
        <div className="search-typeahead-mobile-header">
          <button
            type="button"
            className="search-typeahead-mobile-close"
            aria-label="Close search"
            data-testid="search-typeahead-mobile-close"
            onClick={closeMobileOverlay}
          >
            <X size={20} strokeWidth={1.75} color="#4B4B4B" aria-hidden="true" />
          </button>
          {inputElement}
        </div>
        <div className="search-typeahead-mobile-results">
          {showDropdown && (
            <TypeaheadList
              listboxId={typeahead.listboxId}
              results={typeahead.results}
              activeIndex={typeahead.activeIndex}
              isLoading={typeahead.isLoading}
              query={query}
              onHover={typeahead.setActiveIndex}
              onSelect={navigateToTool}
            />
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={`search-typeahead search-typeahead-${variant}`}>
      {inputElement}
      {showDropdown && (
        <TypeaheadList
          listboxId={typeahead.listboxId}
          results={typeahead.results}
          activeIndex={typeahead.activeIndex}
          isLoading={typeahead.isLoading}
          query={query}
          onHover={typeahead.setActiveIndex}
          onSelect={navigateToTool}
        />
      )}
    </div>
  );
}

/** Re-export hook for ⌘K palette (N2) and other consumers. */
export {
  useToolSearchTypeahead,
  TOOL_SEARCH_DEBOUNCE_MS,
  TOOL_SEARCH_PAGE_SIZE,
  TOOL_SEARCH_MIN_QUERY_LENGTH,
} from "@/hooks/useToolSearchTypeahead";