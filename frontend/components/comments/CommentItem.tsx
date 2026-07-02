"use client";

import type { CommentView } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { ChevronUp } from "lucide-react";

function authBadge(method: string | null): string {
  switch (method) {
    case "github": return "GH";
    case "email": return "Mail";
    case "siwx": return "0x";
    default: return "?";
  }
}

interface CommentItemProps {
  comment: CommentView;
  onUpvote?: (id: string) => void;
  depth?: number;
}

export function CommentItem({ comment, onUpvote, depth = 0 }: CommentItemProps) {
  return (
    <div className={depth > 0 ? "comment-item comment-reply" : "comment-item"}>
      <div className="comment-header">
        <span className="comment-auth-badge">[{authBadge(comment.author_auth_method)}]</span>
        <span className="comment-author">{comment.author_nickname || "Anonymous"}</span>
        {comment.author_is_admin && <span className="comment-admin-badge">Admin</span>}
        <span className="comment-time">{timeAgo(comment.created_at)}</span>
      </div>
      <p className="comment-content">{comment.content}</p>
      <div className="comment-actions">
        <button
          type="button"
          className={comment.viewer_upvoted ? "comment-upvote active" : "comment-upvote"}
          onClick={() => onUpvote?.(comment.id)}
          aria-label="Upvote"
        >
          <ChevronUp size={14} />
          {comment.upvote_count}
        </button>
      </div>
    </div>
  );
}