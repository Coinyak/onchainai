import { ONCHAINAI_MCP_HTTP_URL } from "@/lib/mcp-connect";

/** W8 — paid K2 response shape (documentation / copy examples). */
export interface K2AttributionAnchor {
  anchor_type: string;
  tool_slug: string;
  tool_id: string;
  receipt_id: string;
  probed_at: string;
  endpoint_hash: string;
  note: string;
}

export interface K2ProbeReceipt {
  receipt_id: string;
  probed_at: string;
  endpoint_hash: string;
  live: boolean;
  price_match: boolean;
  advertised_price: string | null;
  actual_price: string | null;
  attribution_anchor: K2AttributionAnchor;
}

export const K2_PROBE_TOOL = "check_endpoint_health";
export const K2_PROBE_PRICE_DISPLAY = "$0.001";
export const K2_REST_PATH = "/api/v2/premium/check-endpoint-health/{slug}";

export const K2_AGENT_SOP_STEPS = [
  "search_tools — find candidate x402 tools (free).",
  "get_tool_detail or compare_tools — check trust_probe stale badge (free, may be up to 24h old).",
  `${K2_PROBE_TOOL} — on-demand live probe + Probe Receipt (x402 ~${K2_PROBE_PRICE_DISPLAY} USDC per call).`,
  "Call the third-party x402 endpoint — attach receipt fields as attestation metadata if your runtime supports it.",
] as const;

export const EXAMPLE_PROBE_RECEIPT: K2ProbeReceipt = {
  receipt_id: "8f3c2a1b-9d4e-4f5a-b6c7-1a2b3c4d5e6f",
  probed_at: "2026-07-07T12:34:56Z",
  endpoint_hash: "a3f91c8e2b104d7f9e6a1c0b5d4e3f2a",
  live: true,
  price_match: true,
  advertised_price: "0.001 usdc",
  actual_price: "1000",
  attribution_anchor: {
    anchor_type: "k1_probe_receipt",
    tool_slug: "example-x402-tool",
    tool_id: "00000000-0000-4000-8000-000000000001",
    receipt_id: "8f3c2a1b-9d4e-4f5a-b6c7-1a2b3c4d5e6f",
    probed_at: "2026-07-07T12:34:56Z",
    endpoint_hash: "a3f91c8e2b104d7f9e6a1c0b5d4e3f2a",
    note: "Attach before third-party x402 call; strengthens attribution evidence, not automatic settlement.",
  },
};

export function exampleK2PaidResponseJson(slug = "example-x402-tool"): string {
  return JSON.stringify(
    {
      data: {
        slug,
        live: true,
        endpoint_verified: true,
        price_verified: true,
        last_probe_at: EXAMPLE_PROBE_RECEIPT.probed_at,
        probe_receipt: EXAMPLE_PROBE_RECEIPT,
        disclaimer:
          "On-demand probe at payment time — liveness and advertised x402 fee match only; not execution cost, safety, or payment guarantee.",
      },
      payment: {
        payer: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
        transaction: "0xabc…",
        price: K2_PROBE_PRICE_DISPLAY,
      },
    },
    null,
    2,
  );
}

export function exampleK2McpCallJson(slug = "example-x402-tool"): string {
  return JSON.stringify(
    {
      jsonrpc: "2.0",
      id: 1,
      method: "tools/call",
      params: {
        name: K2_PROBE_TOOL,
        arguments: { slug },
      },
    },
    null,
    2,
  );
}

export function k2RestProbeUrl(slug: string, origin = "https://www.onchain-ai.xyz"): string {
  return `${origin.replace(/\/$/, "")}/api/v2/premium/check-endpoint-health/${encodeURIComponent(slug)}`;
}

export function k2E2eScriptHint(slug = "goldrush-x402"): string {
  return `EVM_PRIVATE_KEY=0x… node scripts/x402-premium-e2e.mjs ${slug}`;
}

export const K2_MCP_POST_HINT = `POST ${ONCHAINAI_MCP_HTTP_URL} (streamable HTTP, x402-capable client required for paid tool)`;