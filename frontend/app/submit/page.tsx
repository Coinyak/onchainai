"use client";

import { Suspense, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useQuery, useMutation } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { GuestSignInPrompt } from "@/components/auth/GuestSignInPrompt";
import { useAuth } from "@/lib/auth";
import {
  submitTool,
  listMySubmissions,
  probeX402Endpoint,
  submitX402Listing,
  X402_REFERRAL_DISCLOSURE,
  type X402ProbeResponse,
  type X402SubmitResponse,
} from "@/lib/api";

const TOOL_TYPES = ["mcp", "cli", "sdk", "api", "x402", "skill"];

function X402ProbePreview({ probe }: { probe: X402ProbeResponse }) {
  if (!probe.live) {
    return (
      <div
        className="rounded-md border border-border bg-neutral-bg p-4"
        data-testid="x402-probe-failed"
      >
        <p className="text-body-sm font-medium text-error">Endpoint check failed</p>
        <p className="text-body-sm text-secondary mt-1">
          {probe.reason || "The endpoint did not return a valid 402 response."}
        </p>
      </div>
    );
  }
  const d = probe.details;
  return (
    <div
      className="rounded-md border border-border bg-neutral-bg p-4"
      data-testid="x402-probe-live"
    >
      <p className="text-body-sm font-medium text-success">
        Live x402 endpoint — 402 handshake verified
      </p>
      <dl className="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-body-sm">
        {d?.amount && (
          <>
            <dt className="text-secondary">Price</dt>
            <dd>
              {d.amount}
              {d.asset ? ` (${d.asset})` : ""}
            </dd>
          </>
        )}
        {d?.network && (
          <>
            <dt className="text-secondary">Network</dt>
            <dd>{d.network}</dd>
          </>
        )}
        {d?.pay_to && (
          <>
            <dt className="text-secondary">Pay to</dt>
            <dd className="break-all">{d.pay_to}</dd>
          </>
        )}
      </dl>
    </div>
  );
}

function X402SubmitFlow() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [homepage, setHomepage] = useState("");
  const [termsAccepted, setTermsAccepted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<X402SubmitResponse | null>(null);

  const probeMut = useMutation({
    mutationFn: () => probeX402Endpoint(endpointUrl.trim()),
    onError: (e: Error) => setError(e.message),
  });

  const submitMut = useMutation({
    mutationFn: () =>
      submitX402Listing({
        name: name.trim(),
        description: description.trim(),
        endpoint_url: endpointUrl.trim(),
        homepage: homepage.trim() || null,
      }),
    onSuccess: (res) => setResult(res),
    onError: (e: Error) => setError(e.message),
  });

  if (result) {
    return result.published ? (
      <div className="rounded-md border border-border p-6" data-testid="x402-published">
        <h2 className="text-h2 mb-2">Published</h2>
        <p className="text-body-md text-secondary mb-4">
          Your endpoint returned a live 402 handshake and is now listed as a
          community x402 tool.
        </p>
        <Link
          href={`/tools/${result.slug}`}
          className="inline-flex items-center min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium no-underline hover:bg-[#D96400]"
        >
          View your listing
        </Link>
      </div>
    ) : (
      <div className="rounded-md border border-border p-6" data-testid="x402-pending">
        <h2 className="text-h2 mb-2">Held for review</h2>
        <p className="text-body-md text-secondary">
          {result.probe.reason ||
            "The endpoint did not return a valid 402 response."}{" "}
          Your submission was saved and an operator will take a look. Fix the
          endpoint and submit again to publish instantly.
        </p>
      </div>
    );
  }

  return (
    <form
      className="space-y-4"
      data-testid="x402-submit-form"
      onSubmit={(e) => {
        e.preventDefault();
        setError(null);
        submitMut.mutate();
      }}
    >
      <p className="text-body-sm text-secondary rounded-md border border-border bg-neutral-bg p-4">
        x402 listings are checked automatically: paste your payment endpoint
        and we probe it for a valid 402 handshake. A live endpoint publishes
        instantly — no operator queue.
      </p>
      <label className="block">
        <span className="text-body-sm text-secondary">Name</span>
        <input
          required
          minLength={2}
          maxLength={100}
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      <label className="block">
        <span className="text-body-sm text-secondary">Description</span>
        <textarea
          required
          minLength={20}
          maxLength={500}
          className="mt-1 w-full min-h-[120px] p-4 rounded-md border border-border"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </label>
      <label className="block">
        <span className="text-body-sm text-secondary">
          x402 endpoint URL (https, returns 402)
        </span>
        <div className="mt-1 flex gap-2">
          <input
            required
            type="url"
            placeholder="https://api.example.com/x402/quote"
            className="w-full min-h-touch px-4 rounded-md border border-border"
            data-testid="x402-endpoint-input"
            value={endpointUrl}
            onChange={(e) => {
              setEndpointUrl(e.target.value);
              probeMut.reset();
            }}
          />
          <button
            type="button"
            className="min-h-touch px-4 rounded-md border border-border-strong bg-neutral-bg font-medium whitespace-nowrap hover:bg-neutral-surface disabled:opacity-50"
            data-testid="x402-probe-btn"
            disabled={!endpointUrl.trim() || probeMut.isPending}
            onClick={() => {
              setError(null);
              probeMut.mutate();
            }}
          >
            {probeMut.isPending ? "Checking..." : "Check endpoint"}
          </button>
        </div>
      </label>
      {probeMut.data && <X402ProbePreview probe={probeMut.data} />}
      <label className="block">
        <span className="text-body-sm text-secondary">Homepage (optional)</span>
        <input
          type="url"
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
          value={homepage}
          onChange={(e) => setHomepage(e.target.value)}
        />
      </label>
      <label className="flex items-start gap-3 rounded-md border border-border p-4">
        <input
          type="checkbox"
          required
          className="mt-1 h-4 w-4"
          data-testid="x402-terms-checkbox"
          checked={termsAccepted}
          onChange={(e) => setTermsAccepted(e.target.checked)}
        />
        <span className="text-body-sm text-secondary">
          I agree to the open-listing terms: OnchainAI records attribution for
          discovery through this directory and applies the referral rate to
          attributed payment volume. {X402_REFERRAL_DISCLOSURE}
        </span>
      </label>
      <button
        type="submit"
        className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium hover:bg-[#D96400] disabled:opacity-50"
        data-testid="x402-submit-btn"
        disabled={submitMut.isPending || !termsAccepted}
      >
        {submitMut.isPending ? "Probing and publishing..." : "Probe and publish"}
      </button>
      {error && <p className="mt-4 text-error text-body-sm">{error}</p>}
    </form>
  );
}

function SubmitPageContent() {
  const { isAuthenticated } = useAuth();
  const searchParams = useSearchParams();
  const initialType = searchParams.get("type");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [toolType, setToolType] = useState(
    initialType && TOOL_TYPES.includes(initialType) ? initialType : "mcp",
  );
  const functionId = "dev-tool";
  const [repoUrl, setRepoUrl] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const submissionsQuery = useQuery({
    queryKey: ["my-submissions"],
    queryFn: listMySubmissions,
    enabled: isAuthenticated,
  });

  const submitMut = useMutation({
    mutationFn: () =>
      submitTool({
        name,
        description,
        tool_type: toolType,
        function: functionId,
        repo_url: repoUrl || null,
      }),
    onSuccess: (row) => {
      setMessage(`Submission received (status: ${row.status}).`);
      submissionsQuery.refetch();
    },
    onError: (e: Error) => setError(e.message),
  });

  if (!isAuthenticated) {
    return (
      <SiteShell>
        <GuestSignInPrompt
          title="Submit a tool"
          description="Sign in to suggest a crypto tool for operator review."
          testId="submit-sign-in"
        />
      </SiteShell>
    );
  }

  const isX402 = toolType === "x402";

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 max-w-[720px] mx-auto">
        <h1 className="text-h1 mb-2">Submit a tool</h1>
        <p className="text-secondary text-body-md mb-6">
          {isX402
            ? "List your x402 payment endpoint. A live 402 handshake publishes instantly."
            : "Suggest a crypto tool for operator review before it appears publicly."}
        </p>
        <label className="block mb-6">
          <span className="text-body-sm text-secondary">Type</span>
          <select
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            data-testid="submit-type-select"
            value={toolType}
            onChange={(e) => setToolType(e.target.value)}
          >
            {TOOL_TYPES.map((t) => (
              <option key={t} value={t}>
                {t.toUpperCase()}
              </option>
            ))}
          </select>
        </label>

        {isX402 ? (
          <X402SubmitFlow />
        ) : (
          <>
            <form
              className="space-y-4"
              onSubmit={(e) => {
                e.preventDefault();
                setError(null);
                submitMut.mutate();
              }}
            >
              <label className="block">
                <span className="text-body-sm text-secondary">Name</span>
                <input
                  required
                  className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label className="block">
                <span className="text-body-sm text-secondary">Description</span>
                <textarea
                  required
                  className="mt-1 w-full min-h-[120px] p-4 rounded-md border border-border"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </label>
              <label className="block">
                <span className="text-body-sm text-secondary">Repository URL</span>
                <input
                  className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
                  value={repoUrl}
                  onChange={(e) => setRepoUrl(e.target.value)}
                />
              </label>
              <button
                type="submit"
                className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium hover:bg-[#D96400]"
                disabled={submitMut.isPending}
              >
                Submit for review
              </button>
            </form>
            {message && <p className="mt-4 text-success text-body-sm">{message}</p>}
            {error && <p className="mt-4 text-error text-body-sm">{error}</p>}
          </>
        )}

        {submissionsQuery.data && submissionsQuery.data.length > 0 && (
          <section className="mt-12">
            <h2 className="text-h2 mb-4">Your submissions</h2>
            <ul className="divide-y divide-border">
              {submissionsQuery.data.map((s) => (
                <li key={s.id} className="py-3 text-body-md">
                  {s.name} — <span className="text-secondary">{s.status}</span>
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </SiteShell>
  );
}

export default function SubmitPage() {
  return (
    <Suspense fallback={null}>
      <SubmitPageContent />
    </Suspense>
  );
}
