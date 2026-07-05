"use client";

import { useEffect, useRef } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type { PublicTool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { PreviewPanelContent } from "@/components/tools/PreviewPanelContent";
import { PreviewActionBar } from "@/components/tools/PreviewActionBar";

interface PreviewPanelProps {
  tool: PublicTool;
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
  commentCount = 0,
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
}: PreviewPanelProps) {
  const router = useRouter();
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    panelRef.current?.focus({ preventScroll: true });
  }, [tool.slug]);

  useEffect(() => {
    function onKeyDown(ev: KeyboardEvent) {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        router.push(closeHref, { scroll: false });
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [closeHref, router]);

  return (
    <aside
      ref={panelRef}
      className="preview-panel"
      role="complementary"
      aria-label="Tool preview"
      tabIndex={-1}
      data-testid="preview-panel"
    >
      <div className="preview-panel-scroll">
        {addMode ? (
          <>
            <header className="preview-panel-header preview-panel-header--add-mode">
              <h2 className="preview-title">Add MCP: {tool.name}</h2>
              <Link
                href={closeHref}
                scroll={false}
                className="preview-close"
                aria-label="Close preview"
              >
                ×
              </Link>
            </header>
            <ToolDetail
              tool={tool}
              compact
              commentCount={commentCount}
              addMode={addMode}
              addMcpQueryBase={addMcpQueryBase}
              compareReturnHref={compareReturnHref}
            />
          </>
        ) : (
          <PreviewPanelContent
            key={tool.slug}
            tool={tool}
            closeHref={closeHref}
            fullPageHref={fullPageHref}
            commentCount={commentCount}
          />
        )}
      </div>
      {!addMode && (
        <PreviewActionBar
          key={tool.slug}
          tool={tool}
          fullPageHref={fullPageHref}
          addMcpQueryBase={addMcpQueryBase}
        />
      )}
    </aside>
  );
}