"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useMutation } from "@tanstack/react-query";
import { ArrowLeftRight, Plug, Star } from "lucide-react";
import type { Tool } from "@/lib/api";
import { isBookmarked, setBookmark } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import { compareHref } from "@/lib/browser-query";
import {
  addMcpActionLabel,
  addMcpHref,
  toolHasInstallPath,
} from "@/lib/install-guide";

interface PreviewActionBarProps {
  tool: Tool;
  fullPageHref: string;
  addMcpQueryBase?: string;
}

function bookmarkLabel(starred: boolean): string {
  return starred ? "Saved" : "Save";
}

export function PreviewActionBar({
  tool,
  fullPageHref,
  addMcpQueryBase = "",
}: PreviewActionBarProps) {
  const { isAuthenticated } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const [starred, setStarred] = useState(false);
  const hasInstallPath = toolHasInstallPath(tool);
  const addMcpHrefValue =
    addMcpQueryBase && hasInstallPath ? addMcpHref(addMcpQueryBase, tool.slug) : null;
  const addMcpLabel = addMcpActionLabel(tool) ?? "Add MCP";

  useEffect(() => {
    if (!isAuthenticated) return;
    let cancelled = false;
    isBookmarked(tool.slug)
      .then((bookmarked) => {
        if (!cancelled) setStarred(bookmarked);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [isAuthenticated, tool.slug]);

  const bookmarkMut = useMutation({
    mutationFn: (wantStarred: boolean) => setBookmark(tool.slug, wantStarred),
    onSuccess: (_, wantStarred) => setStarred(wantStarred),
    onError: () => setShowLogin(true),
  });

  return (
    <div className="preview-action-bar">
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <Link href={fullPageHref} className="preview-action-primary">
        Open full page
      </Link>
      {addMcpHrefValue ? (
        <Link href={addMcpHrefValue} scroll={false} className="preview-action-btn">
          <Plug size={16} strokeWidth={1.75} aria-hidden />
          <span>{addMcpLabel}</span>
        </Link>
      ) : null}
      <button
        type="button"
        className="preview-action-btn"
        aria-label={bookmarkLabel(starred)}
        aria-pressed={starred}
        onClick={() => {
          if (!isAuthenticated) {
            setShowLogin(true);
            return;
          }
          bookmarkMut.mutate(!starred);
        }}
      >
        <Star
          size={16}
          strokeWidth={1.75}
          fill={starred ? "currentColor" : "none"}
          aria-hidden
        />
        <span>{bookmarkLabel(starred)}</span>
      </button>
      <Link href={compareHref([tool.slug])} className="preview-action-btn">
        <ArrowLeftRight size={16} strokeWidth={1.75} aria-hidden />
        <span>Compare</span>
      </Link>
    </div>
  );
}