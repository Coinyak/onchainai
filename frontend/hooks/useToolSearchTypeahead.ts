"use client";

import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { searchTools, type Tool } from "@/lib/api";

/** Shared constants for hero search, toolbar search, and future ⌘K palette (N2). */
export const TOOL_SEARCH_DEBOUNCE_MS = 200;
export const TOOL_SEARCH_PAGE_SIZE = 5;
export const TOOL_SEARCH_MIN_QUERY_LENGTH = 1;

export interface UseToolSearchTypeaheadOptions {
  query: string;
  debounceMs?: number;
  pageSize?: number;
  minQueryLength?: number;
  functionFilter?: string;
  chainFilter?: string;
  enabled?: boolean;
}

export interface UseToolSearchTypeaheadResult {
  listboxId: string;
  debouncedQuery: string;
  results: Tool[];
  isLoading: boolean;
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
  activeIndex: number;
  setActiveIndex: (index: number) => void;
  activeDescendantId: string | undefined;
  openDropdown: () => void;
  closeDropdown: () => void;
  selectActive: () => Tool | undefined;
  handleKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
}

export function optionId(listboxId: string, slug: string): string {
  return `${listboxId}-option-${slug}`;
}

export function useToolSearchTypeahead({
  query,
  debounceMs = TOOL_SEARCH_DEBOUNCE_MS,
  pageSize = TOOL_SEARCH_PAGE_SIZE,
  minQueryLength = TOOL_SEARCH_MIN_QUERY_LENGTH,
  functionFilter,
  chainFilter,
  enabled = true,
}: UseToolSearchTypeaheadOptions): UseToolSearchTypeaheadResult {
  const listboxId = useId().replace(/:/g, "");
  const [debouncedQuery, setDebouncedQuery] = useState(query);
  const [isOpen, setIsOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setDebouncedQuery(query), debounceMs);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [query, debounceMs]);

  const trimmedDebounced = debouncedQuery.trim();
  const shouldFetch =
    enabled && trimmedDebounced.length >= minQueryLength;

  const searchQuery = useQuery({
    queryKey: [
      "tool-search-typeahead",
      trimmedDebounced,
      pageSize,
      functionFilter,
      chainFilter,
    ],
    queryFn: () =>
      searchTools({
        query: trimmedDebounced,
        function: functionFilter,
        chain: chainFilter,
        page_size: pageSize,
      }),
    enabled: shouldFetch,
    staleTime: 30_000,
  });

  const results = useMemo(() => searchQuery.data ?? [], [searchQuery.data]);

  const resolvedActiveIndex = useMemo(() => {
    if (!isOpen || results.length === 0 || activeIndex < 0) return -1;
    if (activeIndex >= results.length) return results.length - 1;
    return activeIndex;
  }, [activeIndex, isOpen, results.length]);

  const openDropdown = useCallback(() => setIsOpen(true), []);
  const closeDropdown = useCallback(() => {
    setIsOpen(false);
    setActiveIndex(-1);
  }, []);

  const selectActive = useCallback((): Tool | undefined => {
    if (!isOpen || resolvedActiveIndex < 0 || resolvedActiveIndex >= results.length) {
      return undefined;
    }
    return results[resolvedActiveIndex];
  }, [resolvedActiveIndex, isOpen, results]);

  const activeDescendantId = useMemo(() => {
    if (!isOpen || resolvedActiveIndex < 0 || resolvedActiveIndex >= results.length) {
      return undefined;
    }
    return optionId(listboxId, results[resolvedActiveIndex].slug);
  }, [resolvedActiveIndex, isOpen, listboxId, results]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (!isOpen && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
        if (results.length > 0) {
          event.preventDefault();
          setIsOpen(true);
          setActiveIndex(event.key === "ArrowDown" ? 0 : results.length - 1);
        }
        return;
      }

      if (!isOpen) return;

      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          if (results.length === 0) return;
          setActiveIndex((prev) => (prev < results.length - 1 ? prev + 1 : 0));
          break;
        case "ArrowUp":
          event.preventDefault();
          if (results.length === 0) return;
          setActiveIndex((prev) => (prev > 0 ? prev - 1 : results.length - 1));
          break;
        case "Escape":
          event.preventDefault();
          closeDropdown();
          break;
        default:
          break;
      }
    },
    [closeDropdown, isOpen, results.length],
  );

  return {
    listboxId,
    debouncedQuery: trimmedDebounced,
    results,
    isLoading: searchQuery.isFetching,
    isOpen,
    setIsOpen,
    activeIndex: resolvedActiveIndex,
    setActiveIndex,
    activeDescendantId,
    openDropdown,
    closeDropdown,
    selectActive,
    handleKeyDown,
  };
}