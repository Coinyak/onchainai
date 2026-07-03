"use client";

import "./globals.css";
import Link from "next/link";
import { useEffect } from "react";

interface GlobalErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function GlobalErrorPage({ error, reset }: GlobalErrorPageProps) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <html lang="en">
      <body
        className="site-app-shell min-h-screen flex flex-col bg-neutral-bg text-primary font-sans"
        data-testid="global-error-page"
      >
        <main className="site-page-body flex-1 min-h-0 flex flex-col">
          <div className="px-gutter md:px-8 py-10 max-w-[720px] mx-auto">
            <div className="empty-state-panel">
              <h1 style={{ fontSize: "26px", fontWeight: 700, margin: "0 0 12px" }}>
                Something went wrong
              </h1>
              <p style={{ fontSize: "14px", color: "#6B6B6B", margin: "0 0 16px" }}>
                A critical error occurred. Please retry or reload the page.
              </p>
              {error.message && (
                <p style={{ fontSize: "13px", color: "#6B6B6B", margin: "0 0 16px" }}>
                  {error.message}
                </p>
              )}
              <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", justifyContent: "center" }}>
                <button
                  type="button"
                  onClick={reset}
                  data-testid="global-error-retry"
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    justifyContent: "center",
                    height: "36px",
                    padding: "0 16px",
                    borderRadius: "8px",
                    border: "1px solid #E76F00",
                    background: "#E76F00",
                    color: "#FFFFFF",
                    fontSize: "14px",
                    fontWeight: 500,
                    cursor: "pointer",
                  }}
                >
                  Retry
                </button>
                <Link
                  href="/"
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    justifyContent: "center",
                    height: "36px",
                    padding: "0 16px",
                    borderRadius: "8px",
                    border: "1px solid #E5E5E5",
                    background: "#FFFFFF",
                    color: "#1A1A1A",
                    fontSize: "14px",
                    fontWeight: 500,
                    textDecoration: "none",
                  }}
                >
                  Back to home
                </Link>
              </div>
            </div>
          </div>
        </main>
      </body>
    </html>
  );
}