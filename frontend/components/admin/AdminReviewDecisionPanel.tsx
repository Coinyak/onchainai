"use client";

import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { reviewTool } from "@/lib/api";

interface AdminReviewDecisionPanelProps {
  slug: string;
  onReviewed?: () => void;
}

const ACTIONS = [
  { id: "approve", label: "Approve" },
  { id: "reject", label: "Reject" },
  { id: "mark_verified", label: "Mark verified" },
  { id: "mark_official", label: "Mark official" },
  { id: "demote_verified", label: "Demote verified" },
];

export function AdminReviewDecisionPanel({ slug, onReviewed }: AdminReviewDecisionPanelProps) {
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reviewMut = useMutation({
    mutationFn: (action: string) =>
      reviewTool({ slug, action, reason: reason || `Admin action: ${action}` }),
    onSuccess: () => {
      setError(null);
      onReviewed?.();
    },
    onError: (e: Error) => setError(e.message),
  });

  return (
    <section className="admin-review-panel">
      <h3 className="text-h3 mb-3">Review decision</h3>
      <label className="block text-body-sm text-secondary mb-2">Reason / notes</label>
      <textarea
        className="w-full min-h-[80px] p-3 rounded-md border border-border mb-3"
        value={reason}
        onChange={(e) => setReason(e.target.value)}
        placeholder="Operator notes..."
      />
      <div className="admin-review-actions flex flex-wrap gap-2">
        {ACTIONS.map((action) => (
          <button
            key={action.id}
            type="button"
            className="min-h-touch px-4 rounded-md border border-border-strong bg-neutral-bg hover:bg-neutral-hover disabled:opacity-60"
            disabled={reviewMut.isPending}
            onClick={() => reviewMut.mutate(action.id)}
          >
            {action.label}
          </button>
        ))}
      </div>
      {error && <p className="mt-2 text-body-sm text-error">{error}</p>}
    </section>
  );
}