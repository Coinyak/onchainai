"use client";

import Link from "next/link";
import { Plug } from "lucide-react";
import type { Tool } from "@/lib/api";
import {
  addMcpActionLabel,
  addMcpHref,
  addMcpHrefFromCompare,
  toolHasInstallPath,
} from "@/lib/install-guide";

export type AddMcpVariant = "card_icon" | "inline_button" | "detail_primary";

export type AddMcpHrefSource =
  | { kind: "query_base"; base: string }
  | { kind: "compare_slugs"; slugs: string[] };

function resolveHref(source: AddMcpHrefSource, slug: string): string {
  if (source.kind === "query_base") return addMcpHref(source.base, slug);
  return addMcpHrefFromCompare(source.slugs, slug);
}

interface AddMcpActionProps {
  tool: Tool;
  hrefSource: AddMcpHrefSource;
  variant?: AddMcpVariant;
}

export function AddMcpAction({
  tool,
  hrefSource,
  variant = "card_icon",
}: AddMcpActionProps) {
  const label = addMcpActionLabel(tool);
  const hasPath = toolHasInstallPath(tool);
  const href = resolveHref(hrefSource, tool.slug);
  const actionLabel = label ?? "Add MCP";

  if (!hasPath) {
    if (variant === "card_icon") return null;
    return (
      <span className="add-mcp-disabled" aria-disabled="true">
        No install listed
      </span>
    );
  }

  if (variant === "card_icon") {
    return (
      <Link
        href={href}
        className="card-action-btn add-mcp-action"
        aria-label={actionLabel}
        title={actionLabel}
        onClick={(e) => e.stopPropagation()}
      >
        <Plug className="card-action-icon" size={16} strokeWidth={1.75} aria-hidden />
      </Link>
    );
  }

  if (variant === "inline_button") {
    return (
      <Link
        href={href}
        className="add-mcp-inline-btn"
        onClick={(e) => e.stopPropagation()}
      >
        {actionLabel}
      </Link>
    );
  }

  return (
    <Link
      href={href}
      className="add-mcp-primary-btn"
      onClick={(e) => e.stopPropagation()}
    >
      {actionLabel}
    </Link>
  );
}