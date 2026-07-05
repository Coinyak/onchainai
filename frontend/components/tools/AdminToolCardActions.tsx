"use client";

import { useState } from "react";
import Link from "next/link";
import { useMutation } from "@tanstack/react-query";
import { Check, Pencil } from "lucide-react";
import { reviewTool } from "@/lib/api";

const QUICK_VERIFY_REASON = "Verified via public-card admin quick action.";

function canMarkVerified(status: string): boolean {
  return status !== "verified" && status !== "official";
}

interface AdminToolCardActionsProps {
  slug: string;
  status: string;
  onStatusChange?: (status: string) => void;
}

export function AdminToolCardActions({
  slug,
  status,
  onStatusChange,
}: AdminToolCardActionsProps) {
  const [error, setError] = useState<string | null>(null);
  const reviewHref = `/admin/tools?slug=${encodeURIComponent(slug)}&lookup=1`;

  const verifyMut = useMutation({
    mutationFn: () =>
      reviewTool({
        slug,
        action: "mark_verified",
        reason: QUICK_VERIFY_REASON,
      }),
    onSuccess: () => {
      setError(null);
      onStatusChange?.("verified");
    },
    onError: (err: Error) => setError(err.message),
  });

  const handleVerify = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    if (verifyMut.isPending) return;
    if (!window.confirm("Mark this tool verified?")) return;
    verifyMut.mutate();
  };

  return (
    <>
      <Link
        href={reviewHref}
        className="card-action-btn admin-card-action-link"
        aria-label="Review or edit"
        title="Review or edit"
        onClick={(e) => e.stopPropagation()}
      >
        <Pencil className="card-action-icon" size={16} strokeWidth={1.75} aria-hidden />
      </Link>
      {canMarkVerified(status) && (
        <button
          type="button"
          className="card-action-btn admin-card-action-btn"
          aria-label="Mark verified"
          title="Mark verified"
          disabled={verifyMut.isPending}
          onClick={handleVerify}
        >
          <Check className="card-action-icon" size={16} strokeWidth={1.75} aria-hidden />
        </button>
      )}
      {error && (
        <span className="sr-only" role="status">
          {error}
        </span>
      )}
    </>
  );
}