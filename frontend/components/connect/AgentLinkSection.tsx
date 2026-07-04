"use client";

import { useEffect, useId, useMemo, useRef, useState, type FormEvent } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link2, ShieldCheck } from "lucide-react";
import { useAuth } from "@/lib/auth";
import {
  approveAgentDevice,
  getAgentLinkStatus,
  listAgentTokens,
  revokeAgentToken,
} from "@/lib/api";
import { emptyCodeValidationMessage, mapAgentLinkError } from "@/lib/agent-link-copy";
import { AgentLinkClientGuide } from "@/components/connect/AgentLinkClientGuide";

type LinkPhase = "idle" | "approving" | "linked" | "error";

export function AgentLinkSection() {
  const { isAuthenticated } = useAuth();
  const searchParams = useSearchParams();
  const queryClient = useQueryClient();
  const codeFromUrl = searchParams.get("code")?.toUpperCase() ?? "";
  const [userCode, setUserCode] = useState(codeFromUrl);
  const [phase, setPhase] = useState<LinkPhase>("idle");
  const [message, setMessage] = useState<string | null>(null);
  const [messageFocusKey, setMessageFocusKey] = useState(0);
  const [isSuccess, setIsSuccess] = useState(false);
  const [haveCodeOpen, setHaveCodeOpen] = useState(Boolean(codeFromUrl));
  const messageRef = useRef<HTMLParagraphElement>(null);
  const hintId = useId();

  const linkQuery = useQuery({
    queryKey: ["agent-link-status"],
    queryFn: getAgentLinkStatus,
    enabled: isAuthenticated,
  });

  const tokensQuery = useQuery({
    queryKey: ["agent-tokens"],
    queryFn: listAgentTokens,
    enabled: isAuthenticated,
  });

  const linked = linkQuery.data?.linked ?? false;
  const tokens = (tokensQuery.data?.items ?? []).filter((t) => !t.revoked_at);
  const statusLoading =
    isAuthenticated && (linkQuery.isLoading || (linkQuery.isSuccess && tokensQuery.isLoading));
  const statusError = isAuthenticated && (linkQuery.isError || tokensQuery.isError);

  const signInReturnTo = useMemo(
    () =>
      codeFromUrl
        ? `/connect?code=${encodeURIComponent(codeFromUrl)}#agent-sync`
        : "/connect#agent-sync",
    [codeFromUrl],
  );

  useEffect(() => {
    if (message) {
      messageRef.current?.focus();
    }
  }, [message, messageFocusKey]);

  async function handleApprove(e: FormEvent) {
    e.preventDefault();
    if (!userCode.trim()) {
      setPhase("error");
      setIsSuccess(false);
      setMessage(emptyCodeValidationMessage());
      setMessageFocusKey((current) => current + 1);
      return;
    }
    setPhase("approving");
    setMessage(null);
    setIsSuccess(false);
    try {
      const res = await approveAgentDevice(userCode.trim());
      setUserCode("");
      setPhase("linked");
      setIsSuccess(true);
      setMessage(
        res.message ||
          "You're connected! Go back to your coding app — it should finish setup automatically.",
      );
      setMessageFocusKey((current) => current + 1);
      await queryClient.invalidateQueries({ queryKey: ["agent-link-status"] });
      await queryClient.invalidateQueries({ queryKey: ["agent-tokens"] });
    } catch (err) {
      setPhase("error");
      setIsSuccess(false);
      setMessage(mapAgentLinkError(err instanceof Error ? err.message : "Approval failed"));
      setMessageFocusKey((current) => current + 1);
    }
  }

  async function handleRevoke(id: string) {
    try {
      await revokeAgentToken(id);
      await queryClient.invalidateQueries({ queryKey: ["agent-link-status"] });
      await queryClient.invalidateQueries({ queryKey: ["agent-tokens"] });
      if (tokens.length <= 1) setPhase("idle");
    } catch (err) {
      setIsSuccess(false);
      setMessage(mapAgentLinkError(err instanceof Error ? err.message : "Revoke failed"));
      setMessageFocusKey((current) => current + 1);
    }
  }

  const showGuide = !linked || phase === "error";
  const displayPhase = linked || phase === "linked" ? "linked" : phase;

  const header = (
    <>
      <div className="agent-link-header">
        <span className="agent-link-icon" aria-hidden>
          <Link2 size={22} strokeWidth={1.75} />
        </span>
        <div>
          <h2 id="agent-link-heading" className="connect-guide-block-title">
            Save tools from your coding app
          </h2>
          <p className="text-body-sm text-secondary mt-1 max-w-[720px]">
            When you save a tool from a coding agent, it can show up in My Toolkit on this site.
            Connect once per computer — you only type a short code; we never ask you to paste a
            long password.
          </p>
        </div>
      </div>
      <ol className="install-steps agent-link-steps" data-testid="agent-link-steps-guide">
        <li data-testid="agent-link-step-1">
          <strong>Get the code</strong> — run the command for your app (tabs below). A short code
          like K7M3-9P2X appears in the terminal.
        </li>
        <li data-testid="agent-link-step-2">
          <strong>Enter the code here</strong> — open &quot;I already have a code&quot; and click
          Connect.
        </li>
        <li data-testid="agent-link-step-3">
          <strong>See tools in My Toolkit</strong> — saved tools appear with a From agent badge.
        </li>
      </ol>
    </>
  );

  if (!isAuthenticated) {
    return (
      <section
        id="agent-sync"
        className="agent-link-section connect-guide-block"
        aria-labelledby="agent-link-heading"
        data-testid="agent-link-section"
      >
        {header}
        <AgentLinkClientGuide />
        <p className="text-body-sm text-secondary mt-4 mb-3">
          Sign in with GitHub to enter the code from your coding app.
        </p>
        <Link
          href={`/login?return_to=${encodeURIComponent(signInReturnTo)}`}
          className="agent-link-cta"
          data-testid="agent-link-sign-in"
        >
          Sign in to connect
        </Link>
      </section>
    );
  }

  return (
    <section
      id="agent-sync"
      className="agent-link-section connect-guide-block"
      aria-labelledby="agent-link-heading"
      data-testid="agent-link-section"
    >
      {header}

      <div
        className={`agent-link-status ${
          statusError
            ? "agent-link-status--error"
            : linked
              ? "agent-link-status--linked"
              : "agent-link-status--pending"
        }`}
        role="status"
        aria-live="polite"
        data-testid={
          statusError
            ? "agent-link-status-error"
            : linked
              ? "agent-link-status-linked"
              : "agent-link-status-pending"
        }
      >
        <ShieldCheck size={18} aria-hidden />
        <span>
          {statusLoading
            ? "Checking connection…"
            : statusError
              ? "Could not load connection status. Refresh the page to try again."
              : linked
                ? "Connected"
                : "Not connected yet"}
        </span>
      </div>
      {linked && (
        <p className="text-body-sm text-secondary mb-3">
          Your coding app can save tools to My Toolkit.
        </p>
      )}

      {linked && (
        <Link
          href="/toolkit"
          className="agent-link-cta mb-4 inline-flex"
          data-testid="agent-link-open-toolkit"
        >
          Open My Toolkit →
        </Link>
      )}

      {showGuide && <AgentLinkClientGuide />}

      <details
        className="agent-link-have-code mt-4"
        open={haveCodeOpen}
        onToggle={(e) => setHaveCodeOpen((e.target as HTMLDetailsElement).open)}
        data-testid="agent-link-have-code"
      >
        <summary className="agent-link-have-code-summary text-body-md font-semibold cursor-pointer">
          I already have a code
        </summary>
        <fieldset className="agent-link-form mt-3 border-0 p-0 m-0">
          <legend className="sr-only">Enter code from your coding app</legend>
          <form onSubmit={handleApprove}>
            <label htmlFor="agent-user-code" className="agent-link-label">
              Code from your coding app
            </label>
            <div className="agent-link-input-row">
              <input
                id="agent-user-code"
                type="text"
                inputMode="text"
                autoComplete="one-time-code"
                autoCapitalize="characters"
                spellCheck={false}
                placeholder="ABCD-EFGH"
                value={userCode}
                onChange={(e) => setUserCode(e.target.value.toUpperCase())}
                className="agent-link-input"
                data-testid="agent-link-code-input"
                maxLength={9}
                aria-describedby={hintId}
                aria-invalid={phase === "error" && !isSuccess}
              />
              <button
                type="submit"
                className="agent-link-approve"
                disabled={displayPhase === "approving"}
                data-testid="agent-link-approve"
              >
                {displayPhase === "approving" ? "Connecting…" : "Connect"}
              </button>
            </div>
            <p id={hintId} className="text-body-sm text-secondary mt-2">
              8 characters, format XXXX-XXXX (e.g. K7M3-9P2X). Type exactly as shown in your terminal.
            </p>
          </form>
        </fieldset>
      </details>

      {message && (
        <p
          ref={messageRef}
          tabIndex={-1}
          className={`agent-link-message text-body-sm mt-3 ${isSuccess ? "agent-link-banner--success" : "agent-link-banner--error"}`}
          role={isSuccess ? "status" : "alert"}
          data-testid={isSuccess ? "agent-link-success" : "agent-link-error"}
        >
          {message}
        </p>
      )}

      {tokens.length > 0 && (
        <div className="agent-link-tokens mt-4" data-testid="agent-link-connected-apps">
          <h3 className="text-body-md font-semibold mb-2">Connected apps</h3>
          <ul className="agent-link-token-list">
            {tokens.map((t) => (
              <li key={t.id} className="agent-link-token-item">
                <span className="text-body-sm">{t.label}</span>
                <button
                  type="button"
                  className="agent-link-revoke"
                  onClick={() => void handleRevoke(t.id)}
                  aria-label={`Disconnect ${t.label}`}
                  data-testid={`agent-link-disconnect-${t.id}`}
                >
                  Disconnect
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}