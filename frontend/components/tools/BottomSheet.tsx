"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type { Tool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";

interface BottomSheetProps {
  tool: Tool;
  closeHref: string;
  fullPageHref: string;
  commentCount?: number;
  addMode?: boolean;
  addMcpQueryBase?: string;
  compareReturnHref?: string;
}

export function BottomSheet({
  tool,
  closeHref,
  fullPageHref,
  commentCount,
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
}: BottomSheetProps) {
  const router = useRouter();
  const sheetRef = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, []);

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
      <Link href={closeHref} className="bottom-sheet-backdrop" aria-label="Close preview">
        <span className="sr-only">Close</span>
      </Link>
      <div
        ref={sheetRef}
        className={expanded ? "bottom-sheet bottom-sheet-full" : "bottom-sheet"}
        role="dialog"
        aria-label="Tool preview"
        data-testid="preview-bottom-sheet"
      >
        <div
          className="bottom-sheet-handle"
          role="button"
          tabIndex={0}
          aria-label="Drag to expand"
          onClick={() => setExpanded((v) => !v)}
        />
        <div className="bottom-sheet-body">
          <ToolDetail
            tool={tool}
            compact
            commentCount={commentCount}
            addMode={addMode}
            addMcpQueryBase={addMcpQueryBase}
            compareReturnHref={compareReturnHref}
          />
          {!addMode && (
            <Link href={fullPageHref} className="bottom-sheet-view-full">
              View full page
            </Link>
          )}
        </div>
      </div>
    </>
  );
}