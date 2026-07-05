"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Star } from "lucide-react";
import type { PublicTool } from "@/lib/api";
import { isBookmarked, setBookmark } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";

function bookmarkLabel(starred: boolean): string {
  return starred ? "Remove from Toolkit" : "Save to Toolkit";
}

function bookmarkQueryKey(slug: string) {
  return ["bookmark", slug] as const;
}

interface ToolDetailBookmarkProps {
  tool: PublicTool;
}

export function ToolDetailBookmark({ tool }: ToolDetailBookmarkProps) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [showLogin, setShowLogin] = useState(false);

  const bookmarkQuery = useQuery({
    queryKey: bookmarkQueryKey(tool.slug),
    queryFn: () => isBookmarked(tool.slug),
    enabled: isAuthenticated,
  });

  const starred = bookmarkQuery.data ?? false;

  const bookmarkMut = useMutation({
    mutationFn: (wantStarred: boolean) => setBookmark(tool.slug, wantStarred),
    onMutate: async (wantStarred) => {
      await queryClient.cancelQueries({ queryKey: bookmarkQueryKey(tool.slug) });
      const previous = queryClient.getQueryData<boolean>(bookmarkQueryKey(tool.slug));
      queryClient.setQueryData(bookmarkQueryKey(tool.slug), wantStarred);
      return { previous };
    },
    onError: (_err, _wantStarred, context) => {
      if (context?.previous !== undefined) {
        queryClient.setQueryData(bookmarkQueryKey(tool.slug), context.previous);
      }
    },
    onSuccess: (_, wantStarred) => {
      queryClient.setQueryData(bookmarkQueryKey(tool.slug), wantStarred);
    },
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
        disabled={bookmarkMut.isPending || (isAuthenticated && bookmarkQuery.isLoading)}
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