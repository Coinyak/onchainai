"use client";

import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { reviewTool, type Tool } from "@/lib/api";

interface AdminReviewDecisionPanelProps {
  slug: string;
  tool?: Tool;
  onReviewed?: () => void;
}

const ACTIONS = [
  { id: "approve", label: "Approve" },
  { id: "reject", label: "Reject" },
  { id: "mark_verified", label: "Mark verified" },
  { id: "mark_official", label: "Mark official" },
  { id: "demote_verified", label: "Demote verified" },
];

function safeHttpsUrl(url: string | null | undefined): string | null {
  const trimmed = url?.trim();
  if (!trimmed) return null;
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "https:") return null;
    return parsed.href;
  } catch {
    return null;
  }
}

function ToolContext({ tool }: { tool: Tool }) {
  const repoUrl = safeHttpsUrl(tool.repo_url);
  const homepageUrl = safeHttpsUrl(tool.homepage);

  return (
    <div className="admin-review-context mb-4 space-y-3 text-body-sm">
      {tool.description?.trim() && (
        <div>
          <p className="text-secondary mb-1">Description</p>
          <p className="whitespace-pre-wrap">{tool.description.trim()}</p>
        </div>
      )}
      {(repoUrl || homepageUrl) && (
        <div>
          <p className="text-secondary mb-1">Links</p>
          <ul className="space-y-1">
            {repoUrl && (
              <li>
                <a href={repoUrl} className="text-tertiary underline-offset-2 hover:underline" target="_blank" rel="noopener noreferrer">
                  Repository
                </a>
              </li>
            )}
            {homepageUrl && (
              <li>
                <a href={homepageUrl} className="text-tertiary underline-offset-2 hover:underline" target="_blank" rel="noopener noreferrer">
                  Homepage
                </a>
              </li>
            )}
          </ul>
        </div>
      )}
      <div>
        <p className="text-secondary mb-1">Crypto relevance</p>
        <p>
          Score {tool.crypto_relevance_score} · status {tool.relevance_status}
        </p>
        {tool.crypto_relevance_reasons.length > 0 && (
          <ul className="mt-1 list-disc pl-5 text-secondary">
            {tool.crypto_relevance_reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        )}
      </div>
      <div>
        <p className="text-secondary mb-1">Install risk</p>
        <p>
          Level {tool.install_risk_level}
        </p>
        {tool.install_risk_reasons.length > 0 && (
          <ul className="mt-1 list-disc pl-5 text-secondary">
            {tool.install_risk_reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

export function AdminReviewDecisionPanel({ slug, tool, onReviewed }: AdminReviewDecisionPanelProps) {
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
      {tool && <ToolContext tool={tool} />}
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