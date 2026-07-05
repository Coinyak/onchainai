"use client";

import { useState } from "react";
import {
  clientApiBase,
  githubSignInHref,
  githubSwitchHref,
  isVercelPreviewHost,
  productionLoginHref,
} from "@/lib/auth-origin";
import { hardNavigateAfterAuth } from "@/lib/auth-nav";
import { GitHubMarkIcon } from "@/components/icons/GitHubMarkIcon";
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
  const [walletBusy, setWalletBusy] = useState(false);
  const [walletMsg, setWalletMsg] = useState<string | null>(null);

  const apiBase = clientApiBase();
  const githubHref = githubSignInHref();
  const githubSwitchAction = githubSwitchHref();

  const previewHost =
    typeof window !== "undefined" && isVercelPreviewHost(window.location.hostname);

  const headingClass = compact
    ? "text-[18px] font-semibold mb-2"
    : "text-h2 mb-2";
  const descClass = compact
    ? "text-body-md text-secondary mb-6"
    : "text-body-md text-secondary mb-8";

  async function handleWalletSignIn() {
    setWalletBusy(true);
    setWalletMsg(null);
    try {
      const { redirect } = await connectWalletSiwx(apiBase);
      const returnTo = consumeReturnTo();
      const target = returnTo || redirect;
      hardNavigateAfterAuth(target);
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
          You are signed out of OnchainAI. To use a different GitHub account, sign out of GitHub
          from your profile menu (github.com → avatar → Sign out), then continue with GitHub below.
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
          or local dev (<code className="text-body-sm">localhost:3000</code>). Use wallet sign-in
          below to stay authenticated on this preview deployment.
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
      {signedOut && (
        <div className="mt-2 text-center text-body-sm text-secondary">
          Clear your OnchainAI session:{" "}
          <form action={githubSwitchAction} method="post" className="inline">
            <button
              type="submit"
              data-testid="github-switch-account"
              className="text-primary underline hover:no-underline bg-transparent border-0 p-0 cursor-pointer font-inherit text-body-sm"
            >
              Sign out locally
            </button>
          </form>
        </div>
      )}
      <div className="mt-3">
        <button
          type="button"
          data-testid="wallet-sign-in"
          disabled={walletBusy}
          onClick={() => void handleWalletSignIn()}
          className="flex items-center justify-center w-full min-h-touch px-4 py-2.5 rounded-md border border-border text-body-md font-medium hover:bg-neutral-hover disabled:opacity-60 text-primary"
        >
          {walletBusy ? "Connecting wallet..." : "Connect Wallet"}
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