"use client";

import { useState } from "react";
import { API_BASE } from "@/lib/api";

interface LoginFormProps {
  compact?: boolean;
  onCancel?: () => void;
  headingId?: string;
}

export function LoginForm({ compact = false, onCancel, headingId }: LoginFormProps) {
  const [email, setEmail] = useState("");
  const [emailMsg, setEmailMsg] = useState<string | null>(null);
  const [emailBusy, setEmailBusy] = useState(false);

  const headingClass = compact
    ? "text-[18px] font-semibold mb-2"
    : "text-h2 mb-2";
  const descClass = compact
    ? "text-body-md text-secondary mb-6"
    : "text-body-md text-secondary mb-8";

  async function sendMagicLink(e: React.FormEvent) {
    e.preventDefault();
    if (!email.trim()) return;
    setEmailBusy(true);
    setEmailMsg("Sending magic link...");
    try {
      const res = await fetch(`${API_BASE}/auth/email`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email.trim() }),
      });
      setEmailMsg(
        res.ok
          ? "Check your email for the sign-in link."
          : "Could not send magic link. Try again.",
      );
    } catch {
      setEmailMsg("Could not send magic link. Try again.");
    } finally {
      setEmailBusy(false);
    }
  }

  return (
    <div>
      <h1 id={headingId} className={headingClass}>
        Sign in to OnchainAI
      </h1>
      <p className={descClass}>
        Sign in to comment, bookmark tools, and access admin features.
      </p>
      <a
        href={`${API_BASE}/auth/github`}
        rel="external"
        data-testid="github-sign-in"
        className="flex items-center justify-center w-full min-h-touch px-4 py-2.5 rounded-md bg-primary text-white text-body-md font-medium hover:opacity-90 no-underline"
      >
        Continue with GitHub
      </a>
      <p className="mt-2 text-center text-body-sm text-secondary">
        Use a different GitHub account?{" "}
        <form action={`${API_BASE}/auth/github/switch`} method="post" className="inline">
          <button
            type="submit"
            data-testid="github-switch-account"
            className="text-primary underline hover:no-underline bg-transparent border-0 p-0 cursor-pointer font-inherit text-body-sm"
          >
            Sign out of GitHub
          </button>
        </form>
        , then return here and continue with GitHub.
      </p>
      <form className="mt-3 flex gap-2" onSubmit={sendMagicLink}>
        <input
          type="email"
          placeholder="you@example.com"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          className="flex-1 min-h-touch px-4 rounded-md border border-border bg-neutral-bg text-body-md focus:border-tertiary outline-none"
          disabled={emailBusy}
        />
        <button
          type="submit"
          disabled={emailBusy || !email.trim()}
          className="min-h-touch px-4 rounded-md border border-border-strong bg-neutral-bg text-body-md hover:bg-neutral-hover disabled:opacity-60"
        >
          Email
        </button>
      </form>
      {emailMsg && (
        <p className="mt-2 text-body-sm text-secondary" role="status">
          {emailMsg}
        </p>
      )}
      <div className="mt-3">
        <a
          href="/login#wallet"
          data-testid="wallet-sign-in-link"
          className="flex items-center justify-center w-full min-h-touch px-4 py-2.5 rounded-md border border-border text-body-md font-medium hover:bg-neutral-hover no-underline text-primary"
        >
          Connect Wallet (SIWX)
        </a>
      </div>
      {onCancel && (
        <button
          type="button"
          className="mt-4 text-body-sm text-secondary underline"
          onClick={onCancel}
        >
          Cancel
        </button>
      )}
    </div>
  );
}