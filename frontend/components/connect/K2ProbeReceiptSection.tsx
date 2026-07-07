import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import { copyLabelAria } from "@/lib/install-guide";
import {
  EXAMPLE_PROBE_RECEIPT,
  K2_AGENT_SOP_STEPS,
  K2_MCP_POST_HINT,
  K2_PROBE_PRICE_DISPLAY,
  K2_PROBE_TOOL,
  K2_REST_PATH,
  exampleK2McpCallJson,
  exampleK2PaidResponseJson,
  k2E2eScriptHint,
  k2RestProbeUrl,
} from "@/lib/k2-probe-receipt";

function ReceiptField({
  label,
  value,
  mono = true,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="k2-receipt-field">
      <dt className="k2-receipt-field-label">{label}</dt>
      <dd className={mono ? "k2-receipt-field-value k2-receipt-field-value--mono" : "k2-receipt-field-value"}>
        {value}
      </dd>
    </div>
  );
}

function ProbeReceiptCard() {
  const receipt = EXAMPLE_PROBE_RECEIPT;
  const anchor = receipt.attribution_anchor;

  return (
    <div className="k2-receipt-card" data-testid="k2-probe-receipt-example">
      <div className="k2-receipt-card-header">
        <p className="k2-receipt-card-eyebrow">Example Probe Receipt</p>
        <p className="k2-receipt-card-title">
          {receipt.live ? "LIVE" : "Not live"}
          {receipt.price_match ? " · Price match" : " · Price mismatch"}
        </p>
      </div>
      <dl className="k2-receipt-field-grid">
        <ReceiptField label="receipt_id" value={receipt.receipt_id} />
        <ReceiptField label="probed_at" value={receipt.probed_at} />
        <ReceiptField label="endpoint_hash" value={receipt.endpoint_hash} />
        <ReceiptField
          label="advertised vs actual"
          value={`${receipt.advertised_price ?? "—"} / ${receipt.actual_price ?? "—"}`}
        />
        <ReceiptField label="attribution_anchor.type" value={anchor.anchor_type} />
        <ReceiptField label="attribution_anchor.tool_slug" value={anchor.tool_slug} />
        <ReceiptField label="attribution_anchor.note" value={anchor.note} mono={false} />
      </dl>
    </div>
  );
}

export function K2ProbeReceiptSection() {
  const exampleSlug = EXAMPLE_PROBE_RECEIPT.attribution_anchor.tool_slug;
  const paidResponseJson = exampleK2PaidResponseJson(exampleSlug);
  const mcpCallJson = exampleK2McpCallJson(exampleSlug);
  const restUrl = k2RestProbeUrl(exampleSlug);
  const e2eHint = k2E2eScriptHint();

  return (
    <section
      id="k2-probe-receipt"
      className="connect-k2-probe mb-8"
      aria-labelledby="connect-k2-probe-heading"
      data-testid="connect-k2-probe-section"
    >
      <h2 id="connect-k2-probe-heading" className="text-h3 font-semibold mb-3">
        K2 pre-flight probe &amp; Probe Receipt
      </h2>
      <p className="text-secondary text-body-md leading-relaxed mb-4 max-w-[720px]">
        Before calling a third-party x402 endpoint, run{" "}
        <code className="text-code">{K2_PROBE_TOOL}</code> (~{K2_PROBE_PRICE_DISPLAY} USDC per call)
        for an on-demand liveness check and a signed-style{" "}
        <strong className="text-primary font-medium">Probe Receipt</strong>. Discovery tools stay
        free; this is optional insurance metadata — OnchainAI does not connect wallets, custody
        funds, or proxy payments on this page.
      </p>

      <div className="connect-guide-block">
        <h3 className="connect-guide-block-title">Agent 3-step SOP</h3>
        <ol className="install-steps">
          {K2_AGENT_SOP_STEPS.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      </div>

      <div className="connect-k2-probe-grid">
        <ProbeReceiptCard />

        <div className="connect-k2-probe-docs">
          <div className="connect-guide-block">
            <h3 className="connect-guide-block-title">MCP call (x402-capable client)</h3>
            <p className="text-body-sm text-secondary mb-2">{K2_MCP_POST_HINT}</p>
            <div className="tool-install-stack">
              <div className="tool-install">
                <HighlightedCommand command={mcpCallJson} showPrefix={false} showCopy={false} />
                <CopyButton text={mcpCallJson} label={copyLabelAria("Copy MCP call JSON")} />
              </div>
            </div>
            <p className="text-body-sm text-secondary mt-3">
              Claude Code and Cursor may show a connection error on HTTP 402 — that is expected for
              OnchainAI&apos;s K2 probe. Use an external x402-capable HTTP client or the REST path
              below.
            </p>
          </div>

          <div className="connect-guide-block">
            <h3 className="connect-guide-block-title">REST (x402-capable client)</h3>
            <p className="text-body-sm text-secondary mb-2">
              <code className="text-code">GET {K2_REST_PATH}</code> is OnchainAI&apos;s own K2 probe
              endpoint. An x402-capable client receives HTTP 402, settles to OnchainAI, then gets JSON
              with <code className="text-code">data.probe_receipt</code>.
            </p>
            <div className="tool-install-stack">
              <div className="tool-install">
                <HighlightedCommand command={restUrl} showPrefix={false} showCopy={false} />
                <CopyButton text={restUrl} label={copyLabelAria("Copy REST probe URL")} />
              </div>
            </div>
            <p className="text-body-sm text-secondary mt-3">
              Repo e2e: <code className="text-code">{e2eHint}</code>
            </p>
          </div>

          <div className="connect-guide-block">
            <h3 className="connect-guide-block-title">Full paid response (example)</h3>
            <div className="tool-install-stack">
              <div className="tool-install">
                <HighlightedCommand
                  command={paidResponseJson}
                  showPrefix={false}
                  showCopy={false}
                />
                <CopyButton
                  text={paidResponseJson}
                  label={copyLabelAria("Copy example paid response")}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <p className="text-body-sm text-secondary mt-4 max-w-[720px]">
        The <code className="text-code">attribution_anchor</code> field strengthens K1 referral
        evidence when you attach the receipt before a third-party call. It does not auto-settle or
        move funds. Free <code className="text-code">trust_probe</code> badges on tool pages show
        stale catalog data; only a paid probe mints a fresh receipt.
      </p>
    </section>
  );
}