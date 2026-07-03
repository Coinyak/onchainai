"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { getToolComments } from "@/lib/api";
import { CommentItem } from "@/components/comments/CommentItem";

interface PreviewCommentsPreviewProps {
  slug: string;
  commentCount: number;
  fullPageHref: string;
}

export function PreviewCommentsPreview({
  slug,
  commentCount,
  fullPageHref,
}: PreviewCommentsPreviewProps) {
  const { data: comments, isLoading } = useQuery({
    queryKey: ["comments", slug, "new"],
    queryFn: () => getToolComments(slug, "new"),
  });

  const latest = (comments ?? []).find((comment) => !comment.parent_id);

  return (
    <section className="preview-comments" aria-label="Comments preview">
      <h3 className="preview-section-heading">Comments ({commentCount})</h3>
      {isLoading ? (
        <p className="preview-comments-loading text-body-sm text-secondary">Loading comments…</p>
      ) : latest ? (
        <div className="preview-comment-preview">
          <CommentItem comment={latest} />
        </div>
      ) : (
        <p className="preview-comments-empty text-body-sm text-secondary">No comments yet.</p>
      )}
      <Link href={`${fullPageHref}#comments`} className="preview-comments-link">
        View all comments →
      </Link>
    </section>
  );
}