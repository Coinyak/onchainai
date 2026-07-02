"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { searchTools } from "@/lib/api";

interface ToolFinderProps {
  query: string;
}

export function ToolFinder({ query }: ToolFinderProps) {
  const { data, isLoading } = useQuery({
    queryKey: ["tool-finder", query],
    queryFn: () => searchTools({ query }),
    enabled: query.trim().length >= 2,
  });

  if (!query.trim() || query.trim().length < 2) return null;

  return (
    <div className="tool-finder-panel">
      {isLoading && <p className="text-body-sm text-secondary">Searching...</p>}
      {data && data.length === 0 && (
        <p className="text-body-sm text-secondary">No quick matches.</p>
      )}
      {data && data.length > 0 && (
        <ul className="tool-finder-list">
          {data.slice(0, 5).map((tool) => (
            <li key={tool.slug}>
              <Link href={`/tools/${tool.slug}`} className="tool-finder-item no-underline">
                {tool.name}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function ToolFinderPanel() {
  return null;
}