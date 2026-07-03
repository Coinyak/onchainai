"use client";

import { useCallback } from "react";
import { useRouter } from "next/navigation";
import { ToolSearchCombobox } from "@/components/tools/ToolSearchCombobox";

interface SearchBarProps {
  /** Echo active `q` query param in the input (Phase 3 search mode). */
  defaultValue?: string;
  /** Base path for full search submit. Defaults to home (`/`). */
  searchPath?: string;
}

export function SearchBar({ defaultValue = "", searchPath = "/" }: SearchBarProps) {
  const router = useRouter();

  const handleSubmitSearch = useCallback(
    (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) return;
      const separator = searchPath.includes("?") ? "&" : "?";
      router.push(`${searchPath}${separator}q=${encodeURIComponent(trimmed)}`, {
        scroll: false,
      });
    },
    [router, searchPath],
  );

  const handleDebouncedQueryChange = useCallback(
    (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) {
        if (defaultValue) router.replace(searchPath, { scroll: false });
        return;
      }
      const separator = searchPath.includes("?") ? "&" : "?";
      const target = `${searchPath}${separator}q=${encodeURIComponent(trimmed)}`;
      router.replace(target, { scroll: false });
    },
    [defaultValue, router, searchPath],
  );

  return (
    <ToolSearchCombobox
      variant="hero"
      defaultValue={defaultValue}
      onSubmitSearch={handleSubmitSearch}
      onDebouncedQueryChange={handleDebouncedQueryChange}
      data-testid="home-search-bar"
    />
  );
}