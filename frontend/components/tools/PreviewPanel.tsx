"use client";

import Link from "next/link";
import { X } from "lucide-react";
import type { Tool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { CommentsSection } from "@/components/comments/CommentsSection";

interface PreviewPanelProps {
  tool: Tool;
  closeHref: string;
  fullPageHref: string;
  commentCount?: number;
}

export function PreviewPanel({ tool, closeHref, fullPageHref, commentCount }: PreviewPanelProps) {
  return (
    <aside className="preview-panel" aria-label={`Preview: ${tool.name}`}>
      <div className="preview-panel-header">
        <Link href={fullPageHref} className="preview-open-link text-body-sm">
          Open full page
        </Link>
        <Link href={closeHref} className="preview-close-btn" aria-label="Close preview">
          <X size={20} />
        </Link>
      </div>
      <div className="preview-panel-body">
        <ToolDetail tool={tool} compact commentCount={commentCount} />
        <CommentsSection slug={tool.slug} compact />
      </div>
    </aside>
  );
}