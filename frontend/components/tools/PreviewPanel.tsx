"use client";

import { useEffect, useRef } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type { Tool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { CommentsSection } from "@/components/comments/CommentsSection";

interface PreviewPanelProps {
  tool: Tool;
  closeHref: string;
  fullPageHref: string;
  commentCount?: number;
  addMode?: boolean;
  addMcpQueryBase?: string;
  compareReturnHref?: string;
}

export function PreviewPanel({
  tool,
  closeHref,
  fullPageHref,
  commentCount,
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
}: PreviewPanelProps) {
  const router = useRouter();
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    panelRef.current?.focus();
  }, [tool.slug]);

  useEffect(() => {
    function onKeyDown(ev: KeyboardEvent) {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        router.push(closeHref);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [closeHref, router]);

  return (
    <>
      <Link href={closeHref} className="preview-backdrop" aria-label="Close preview">
        <span className="sr-only">Close</span>
      </Link>
      <aside
        ref={panelRef}
        className="preview-panel"
        role="dialog"
        aria-label="Tool preview"
        tabIndex={-1}
        data-testid="preview-panel"
      >
        <div className="preview-panel-header">
          <Link href={closeHref} className="preview-close" aria-label="Close preview">
            ×
          </Link>
        </div>
        <div className="preview-panel-body">
          <ToolDetail
            tool={tool}
            compact
            commentCount={commentCount}
            addMode={addMode}
            addMcpQueryBase={addMcpQueryBase}
            compareReturnHref={compareReturnHref}
          />
          {!addMode && <CommentsSection slug={tool.slug} compact />}
        </div>
      </aside>
    </>
  );
}