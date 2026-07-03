"use client";

import Link from "next/link";
import { useEffect } from "react";

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function ErrorPage({ error, reset }: ErrorPageProps) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <div className="px-gutter md:px-8 py-10 max-w-[720px] mx-auto" data-testid="error-page">
      <div className="empty-state-panel">
        <h1 className="text-h1 mb-3">Something went wrong</h1>
        <p className="empty-state-message">
          We could not load this page. You can retry or return to the directory.
        </p>
        {error.message && (
          <p className="empty-state-hint">{error.message}</p>
        )}
        <div className="empty-state-actions">
          <button
            type="button"
            className="empty-state-submit-btn"
            onClick={reset}
            data-testid="error-retry"
          >
            Retry
          </button>
          <Link href="/" className="empty-state-clear-btn">
            Back to home
          </Link>
        </div>
      </div>
    </div>
  );
}