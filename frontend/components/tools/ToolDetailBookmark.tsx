"use client";

import { useEffect, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Star } from "lucide-react";
import type { PublicTool } from "@/lib/api";
import { isBookmarked, setBookmark } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";

function bookmarkLabel(starred: boolean): string {
  return starred ? "Remove from Toolkit" : "Save to Toolkit";
}

interface ToolDetailBookmarkProps {
  tool: PublicTool;
}

export function ToolDetailBookmark({ tool }: ToolDetailBookmarkProps) {
  const { isAuthenticated } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const [starred, setStarred] = useState(false);

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
    <>
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <button
        type="button"
        className="preview-action-btn"
        aria-label={bookmarkLabel(starred)}
        aria-pressed={starred}
        data-testid="tool-detail-bookmark"
        onClick={() => {
          if (!isAuthenticated) {
            setShowLogin(true);
            return;
          }
          bookmarkMut.mutate(!starred);
        }}
      >
        <Star
          size={18}
          strokeWidth={1.75}
          fill={starred ? "currentColor" : "none"}
          aria-hidden
        />
        <span>{bookmarkLabel(starred)}</span>
      </button>
    </>
  );
}