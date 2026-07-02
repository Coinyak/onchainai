"use client";

import { useState } from "react";

interface CommentFormProps {
  onSubmit: (content: string) => Promise<void>;
  onLoginRequired?: () => void;
  isAuthenticated: boolean;
  placeholder?: string;
}

export function CommentForm({
  onSubmit,
  onLoginRequired,
  isAuthenticated,
  placeholder = "Write a comment...",
}: CommentFormProps) {
  const [content, setContent] = useState("");
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!isAuthenticated) {
      onLoginRequired?.();
      return;
    }
    const trimmed = content.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      await onSubmit(trimmed);
      setContent("");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="comment-form" onSubmit={handleSubmit}>
      <textarea
        className="comment-input"
        placeholder={placeholder}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        onFocus={() => {
          if (!isAuthenticated) onLoginRequired?.();
        }}
        rows={2}
        maxLength={2000}
      />
      <button
        type="submit"
        className="comment-submit-btn"
        disabled={busy || !content.trim()}
      >
        Post
      </button>
    </form>
  );
}