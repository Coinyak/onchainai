"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  getToolComments,
  createComment,
  toggleUpvote,
  type CommentView,
} from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import { CommentItem } from "@/components/comments/CommentItem";
import { CommentForm } from "@/components/comments/CommentForm";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

interface CommentsSectionProps {
  slug: string;
  toolName?: string;
  compact?: boolean;
}

export function CommentsSection({ slug, compact = false }: CommentsSectionProps) {
  const { isAuthenticated } = useAuth();
  const [sort, setSort] = useState<"new" | "top">("new");
  const [showLogin, setShowLogin] = useState(false);
  const queryClient = useQueryClient();

  const { data: comments, isLoading } = useQuery({
    queryKey: ["comments", slug, sort],
    queryFn: () => getToolComments(slug, sort),
  });

  const createMut = useMutation({
    mutationFn: (content: string) => createComment(slug, content),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["comments", slug] }),
  });

  const upvoteMut = useMutation({
    mutationFn: (id: string) => toggleUpvote(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["comments", slug] }),
  });

  const topLevel = (comments ?? []).filter((c) => !c.parent_id);
  const repliesByParent = (comments ?? []).reduce<Record<string, CommentView[]>>((acc, c) => {
    if (c.parent_id) {
      (acc[c.parent_id] ??= []).push(c);
    }
    return acc;
  }, {});

  return (
    <section className="detail-section comments-section">
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <div className="comments-header">
        <h2 className="text-h2">Comments ({comments?.length ?? 0})</h2>
        {!compact && (
          <select
            className="comments-sort"
            value={sort}
            onChange={(e) => setSort(e.target.value as "new" | "top")}
            aria-label="Sort comments"
          >
            <option value="new">New</option>
            <option value="top">Top</option>
          </select>
        )}
      </div>

      {isLoading ? (
        <ToolListSkeleton count={2} />
      ) : (
        <div className="comments-list">
          {topLevel.map((comment) => (
            <div key={comment.id}>
              <CommentItem
                comment={comment}
                onUpvote={isAuthenticated ? (id) => upvoteMut.mutate(id) : () => setShowLogin(true)}
              />
              {(repliesByParent[comment.id] ?? []).map((reply) => (
                <CommentItem
                  key={reply.id}
                  comment={reply}
                  depth={1}
                  onUpvote={isAuthenticated ? (id) => upvoteMut.mutate(id) : () => setShowLogin(true)}
                />
              ))}
            </div>
          ))}
        </div>
      )}

      <CommentForm
        isAuthenticated={isAuthenticated}
        onLoginRequired={() => setShowLogin(true)}
        onSubmit={async (content) => {
          await createMut.mutateAsync(content);
        }}
      />
    </section>
  );
}