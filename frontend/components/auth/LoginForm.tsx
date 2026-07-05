"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { clientApiBase, githubSignInHref, githubSwitchHref, googleSignInHref, isVercelPreviewHost, productionLoginHref } from "@/lib/auth-origin";
import { useAuth } from "@/lib/auth";
import { getAuthProviders } from "@/lib/api";
import { GitHubMarkIcon } from "@/components/icons/GitHubMarkIcon";
import { GoogleMarkIcon } from "@/components/icons/GoogleMarkIcon";
import { connectWalletSiwx, SiwxError } from "@/lib/siwx";
import { consumeReturnTo } from "@/lib/return-to";

interface LoginFormProps {
  compact?: boolean;
  onCancel?: () => void;
  headingId?: string;
  authError?: string | null;
  signedOut?: boolean;
}

export function LoginForm({
  compact = false,
  onCancel,
  headingId = "login-title",
  authError = null,
  signedOut = false,
}: LoginFormProps) {
  const queryClient = useQueryClient();
  const { data: providers } = useQuery({
    queryKey: ["auth-providers"],
    queryFn: getAuthProviders,
    staleTime: 5 * 60 * 1000,
  });
  const [email, setEmail] = useState("");
  const [emailMsg, setEmailMsg] = useState<string | null>(null);
  const [emailBusy, setEmailBusy] = useState(false);
  const [walletBusy, setWalletBusy] = useState(false);
  const [walletMsg, setWalletMsg] = useState<string | null>(null);

  const apiBase = clientApiBase();
  const githubHref = githubSignInHref();
  const githubSwitchAction = githubSwitchHref();
  const googleHref = googleSignInHref();
  const previewHost =
    typeof window !== "undefined" && isVercelPreviewHost(window.location.hostname);

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
      const res = await fetch(`${apiBase}/auth/email`, {
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

  async function handleWalletSignIn() {
    setWalletBusy(true);
    setWalletMsg(null);
    try {
      const { redirect } = await connectWalletSiwx(apiBase);
      const returnTo = consumeReturnTo();
      queryClient.removeQueries({ queryKey: ["me"] });
      window.location.assign(returnTo || redirect);
    } catch (err) {
      const message =
        err instanceof SiwxError
          ? err.message
          : "Wallet sign-in failed. Try again.";
      setWalletMsg(message);
    } finally {
      setWalletBusy(false);
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
      {authError && (
        <p
          className="mb-4 rounded-md border border-error/30 bg-error/5 px-4 py-3 text-body-sm text-error"
          role="alert"
          data-testid="auth-error-banner"
        >
          {authError}
        </p>
      )}
      {signedOut && (
        <p
          className="mb-4 rounded-md border border-border px-4 py-3 text-body-sm text-secondary"
          role="status"
          data-testid="signed-out-notice"
        >
          You are signed out of OnchainAI. If &quot;Continue with GitHub&quot; signs you back in
          immediately, GitHub still has an active session — use{" "}
          <strong>Sign out of GitHub</strong> below first, then sign in again.
        </p>
      )}
      {previewHost && (
        <p
          className="mb-4 rounded-md border border-border bg-neutral-hover px-4 py-3 text-body-sm text-secondary"
          role="status"
          data-testid="preview-auth-notice"
        >
          GitHub sign-in does not work on Vercel preview URLs — OAuth callbacks are registered
          for production only. Use{" "}
          <a href={productionLoginHref()} className="text-primary underline hover:no-underline">
            www.onchain-ai.xyz
          </a>{" "}
          or local dev (<code className="text-body-sm">localhost:3000</code>). Use wallet or email
          sign-in below to stay authenticated on this preview deployment.
        </p>
      )}
      <a
        href={githubHref}
        rel="external"
        data-testid="github-sign-in"
        className="flex items-center justify-center gap-2 w-full min-h-touch px-4 py-2.5 rounded-md bg-primary text-white text-body-md font-medium hover:opacity-90 no-underline"
      >
        <GitHubMarkIcon size={18} className="shrink-0" />
        Continue with GitHub
      </a>
      <div className="mt-2 text-center text-body-sm text-secondary">
        Use a different GitHub account?{" "}
        <form action={githubSwitchAction} method="post" className="inline">
          <button
            type="submit"
            data-testid="github-switch-account"
            className="text-primary underline hover:no-underline bg-transparent border-0 p-0 cursor-pointer font-inherit text-body-sm"
          >
            Sign out of GitHub
          </button>
        </form>
        , then return here and continue with GitHub.
      </div>
      {providers?.google ? (
        <a
          href={googleHref}
          rel="external"
          data-testid="google-sign-in"
          className="mt-3 flex items-center justify-center gap-2 w-full min-h-touch px-4 py-2.5 rounded-md border border-border-strong bg-neutral-bg text-body-md font-medium hover:bg-neutral-hover no-underline text-primary"
        >
          <GoogleMarkIcon size={18} className="shrink-0" />
          Continue with Google
        </a>
      ) : (
        <p
          className="mt-3 rounded-md border border-border bg-neutral-hover px-4 py-3 text-body-sm text-secondary"
          data-testid="google-sign-in-unavailable"
        >
          Google sign-in is not configured on this deployment yet. Use GitHub, wallet, or email.
        </p>
      )}
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
        <button
          type="button"
          data-testid="wallet-sign-in"
          disabled={walletBusy}
          onClick={() => void handleWalletSignIn()}
          className="flex items-center justify-center w-full min-h-touch px-4 py-2.5 rounded-md border border-border text-body-md font-medium hover:bg-neutral-hover disabled:opacity-60 text-primary"
        >
          {walletBusy ? "Connecting wallet..." : "Connect Wallet (SIWX)"}
        </button>
      </div>
      {walletMsg && (
        <p className="mt-2 text-body-sm text-error" role="alert">
          {walletMsg}
        </p>
      )}
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