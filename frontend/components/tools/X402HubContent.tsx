"use client";

import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { ConnectOnchainAiMcpCard } from "@/components/tools/ConnectOnchainAiMcpCard";
import { listTools, X402_REFERRAL_DISCLOSURE, type PublicToolSummary } from "@/lib/api";

const HOW_IT_WORKS = [
  {
    title: "Paste your endpoint",
    body: "Any https URL that answers with an x402 402 Payment Required handshake qualifies. No SDK, no partner program.",
  },
  {
    title: "We probe it",
    body: "The listing is verified by machine: a live 402 response with parseable payment requirements publishes instantly.",
  },
  {
    title: "Agents find and pay you",
    body: "Your endpoint becomes discoverable to agents through the site, the OnchainAI MCP server, and the plugin.",
  },
];

function X402ToolRow({ tool }: { tool: PublicToolSummary }) {
  return (
    <li className="py-4 flex flex-col gap-1 min-w-0">
      <div className="flex items-baseline gap-3 min-w-0">
        <Link
          href={`/tools/${tool.slug}`}
          className="text-body-md font-semibold text-primary no-underline hover:underline truncate"
        >
          {tool.name}
        </Link>
        {tool.x402_price && (
          <span className="text-body-sm text-secondary min-w-0">
            {tool.x402_price}
          </span>
        )}
      </div>
      {tool.description && (
        <p className="text-body-sm text-secondary line-clamp-2">{tool.description}</p>
      )}
    </li>
  );
}

export function X402HubContent() {
  const toolsQuery = useQuery({
    queryKey: ["x402-tools"],
    queryFn: () =>
      listTools({
        sort: "new",
        offset: 0,
        limit: 12,
        filters: { tool_type: ["x402"] },
      }),
  });
  const tools = toolsQuery.data ?? [];

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 md:py-10 max-w-[880px] mx-auto">
        <section className="mb-10">
          <h1 className="text-h1 md:text-[36px] font-bold tracking-tight leading-tight mb-3">
            x402 tools, machine-checked
          </h1>
          <p className="text-secondary text-body-md leading-relaxed max-w-[640px] mb-6">
            x402 lets AI agents pay for API calls over HTTP with stablecoins —
            no accounts, no API keys. This directory lists live x402 endpoints
            and verifies each one by probing its 402 payment handshake.
          </p>
          <div className="flex flex-wrap gap-3">
            <Link
              href="/submit?type=x402"
              className="inline-flex items-center min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium no-underline hover:bg-[#D96400]"
              data-testid="x402-hub-list-cta"
            >
              List your endpoint
            </Link>
            <Link
              href="/tools?type=x402"
              className="inline-flex items-center min-h-touch px-6 rounded-md border border-border-strong bg-neutral-bg text-primary font-medium no-underline hover:bg-neutral-surface"
            >
              Browse all x402 tools
            </Link>
          </div>
        </section>

        <section className="mb-10">
          <h2 className="text-h2 mb-4">How listing works</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {HOW_IT_WORKS.map((step, i) => (
              <div key={step.title} className="rounded-lg border border-border p-5">
                <p className="text-body-sm text-secondary mb-1">Step {i + 1}</p>
                <h3 className="text-body-md font-semibold mb-2">{step.title}</h3>
                <p className="text-body-sm text-secondary leading-relaxed">{step.body}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="mb-10" data-testid="x402-hub-live-list">
          <h2 className="text-h2 mb-4">Live x402 endpoints</h2>
          {tools.length > 0 ? (
            <ul className="divide-y divide-border rounded-lg border border-border px-5">
              {tools.map((tool) => (
                <X402ToolRow key={tool.slug} tool={tool} />
              ))}
            </ul>
          ) : (
            <div className="rounded-lg border border-border p-6">
              <p className="text-body-md text-secondary">
                {toolsQuery.isLoading
                  ? "Loading live endpoints..."
                  : "No x402 endpoints listed yet. Be the first — a live 402 handshake publishes instantly."}
              </p>
            </div>
          )}
        </section>

        <section className="mb-10">
          <h2 className="text-h2 mb-4">Find x402 tools from your agent</h2>
          <ConnectOnchainAiMcpCard />
        </section>

        <p className="text-body-sm text-secondary border-t border-border pt-4">
          {X402_REFERRAL_DISCLOSURE}
        </p>
      </div>
    </SiteShell>
  );
}
