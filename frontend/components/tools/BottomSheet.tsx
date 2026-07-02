"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { X, ExternalLink } from "lucide-react";
import type { Tool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";

interface BottomSheetProps {
  tool: Tool;
  closeHref: string;
  fullPageHref: string;
  commentCount?: number;
}

export function BottomSheet({ tool, closeHref, fullPageHref, commentCount }: BottomSheetProps) {
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, []);

  return (
    <>
      <Link href={closeHref} className="bottom-sheet-backdrop" aria-label="Close preview" />
      <div className={`bottom-sheet ${expanded ? "bottom-sheet-full" : ""}`}>
        <div
          className="bottom-sheet-handle"
          role="button"
          tabIndex={0}
          aria-label="Drag to expand"
          onClick={() => setExpanded((v) => !v)}
        />
        <div className="bottom-sheet-header">
          <Link href={closeHref} className="bottom-sheet-close" aria-label="Close">
            <X size={20} />
          </Link>
          <Link href={fullPageHref} className="bottom-sheet-open">
            <ExternalLink size={16} /> Open
          </Link>
        </div>
        <div className="bottom-sheet-body">
          <ToolDetail tool={tool} compact commentCount={commentCount} />
          {!expanded && (
            <Link href={fullPageHref} className="bottom-sheet-view-full">
              View full page
            </Link>
          )}
        </div>
      </div>
    </>
  );
}