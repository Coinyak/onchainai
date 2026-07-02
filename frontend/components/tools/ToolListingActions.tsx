"use client";

import { useState } from "react";
import Link from "next/link";
import { useMutation } from "@tanstack/react-query";
import type { Tool } from "@/lib/api";
import { reportTool } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";

interface ToolListingActionsProps {
  tool: Tool;
}

export function ToolListingActions({ tool }: ToolListingActionsProps) {
  const { isAuthenticated } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const [reportReason, setReportReason] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  const reportMut = useMutation({
    mutationFn: () => reportTool(tool.slug, reportReason),
    onSuccess: () => setMessage("Report submitted. Operators will review."),
  });

  return (
    <section className="tool-listing-actions detail-section">
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <h2 className="text-h2 mb-3">Listing actions</h2>
      <div className="listing-actions-row">
        <Link href={`/compare?slugs=${tool.slug}`} className="listing-action-link">
          Compare
        </Link>
        <Link href="/submit" className="listing-action-link">
          Suggest similar
        </Link>
      </div>
      <div className="listing-report mt-4">
        <label className="block text-body-sm text-secondary mb-2">Report an issue</label>
        <textarea
          className="w-full min-h-[80px] p-3 rounded-md border border-border text-body-md"
          placeholder="Describe the problem..."
          value={reportReason}
          onChange={(e) => setReportReason(e.target.value)}
        />
        <button
          type="button"
          className="mt-2 min-h-touch px-4 rounded-md border border-border-strong bg-neutral-bg hover:bg-neutral-hover"
          onClick={() => {
            if (!isAuthenticated) {
              setShowLogin(true);
              return;
            }
            if (reportReason.trim()) reportMut.mutate();
          }}
        >
          Submit report
        </button>
        {message && <p className="mt-2 text-body-sm text-success">{message}</p>}
      </div>
    </section>
  );
}