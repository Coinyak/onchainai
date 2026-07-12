/**
 * Settle each CDP multi-price seller SKU once (for Bazaar indexing).
 *
 * Usage:
 *   EVM_PRIVATE_KEY=0x... node scripts/x402-cdp-seller-settle-all.mjs [api_base]
 *
 * Expects OKX gate OFF so CDP/Base USDC 402 is returned.
 * Defaults api_base: https://www.onchain-ai.xyz
 */
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { registerExactEvmScheme } from "@x402/evm/exact/client";
import { privateKeyToAccount } from "viem/accounts";

const apiBase = (process.argv[2] || "https://www.onchain-ai.xyz").replace(/\/$/, "");
const privateKey = process.env.EVM_PRIVATE_KEY;
if (!privateKey || !/^0x[0-9a-fA-F]{64}$/.test(privateKey)) {
  console.error("Set EVM_PRIVATE_KEY (0x + 64 hex) — Base mainnet USDC + ETH for gas.");
  process.exit(1);
}

/** @type {{ name: string, method: string, url: string, body?: object }[]} */
const skus = [
  {
    name: "check_endpoint_health $0.001",
    method: "GET",
    url: `${apiBase}/api/v2/premium/check-endpoint-health/goldrush-x402`,
  },
  {
    name: "recommend_verified_tool $0.01",
    method: "POST",
    url: `${apiBase}/api/v2/premium/recommend-verified-tool`,
    body: { intent: "bridge USDC to Base", chain: "base" },
  },
  {
    name: "gap_audit $0.01",
    method: "POST",
    url: `${apiBase}/api/v2/premium/gap-audit`,
    body: { intent: "bridge then swap to USDC and stake" },
  },
];

const signer = privateKeyToAccount(privateKey);
const client = new x402Client();
registerExactEvmScheme(client, { signer });
const fetchWithPayment = wrapFetchWithPayment(fetch, client);
const httpClient = new x402HTTPClient(client);

console.log(`buyer ${signer.address}`);
console.log(`api   ${apiBase}`);

const results = [];
for (const sku of skus) {
  console.log(`\n== ${sku.name} ==`);
  const unpaidInit = {
    method: sku.method,
    headers: { Accept: "application/json", "Content-Type": "application/json" },
    body: sku.body ? JSON.stringify(sku.body) : undefined,
  };
  const unpaid = await fetch(sku.url, unpaidInit);
  console.log(`  unpaid ${unpaid.status}`);
  if (unpaid.status !== 402) {
    const t = await unpaid.text();
    results.push({ sku: sku.name, ok: false, reason: `expected 402 got ${unpaid.status}`, body: t.slice(0, 200) });
    continue;
  }
  const pr = unpaid.headers.get("payment-required");
  if (!pr) {
    results.push({ sku: sku.name, ok: false, reason: "missing PAYMENT-REQUIRED" });
    continue;
  }
  // Decode network for sanity
  try {
    const decoded = JSON.parse(Buffer.from(pr, "base64").toString("utf8"));
    const net = decoded?.accepts?.[0]?.network;
    const amt = decoded?.accepts?.[0]?.amount;
    const res = decoded?.accepts?.[0]?.resource?.url || decoded?.resource?.url;
    console.log(`  network=${net} amount=${amt}`);
    console.log(`  resource=${res}`);
    if (net && net !== "eip155:8453") {
      results.push({
        sku: sku.name,
        ok: false,
        reason: `not Base CDP (network=${net}) — disable OKX gate first`,
      });
      continue;
    }
  } catch {
    /* ignore decode */
  }

  const paid = await fetchWithPayment(sku.url, {
    method: sku.method,
    headers: { Accept: "application/json", "Content-Type": "application/json" },
    body: sku.body ? JSON.stringify(sku.body) : undefined,
  });
  const bodyText = await paid.text();
  let body;
  try {
    body = JSON.parse(bodyText);
  } catch {
    body = bodyText.slice(0, 300);
  }
  const settle = httpClient.getPaymentSettleResponse((name) => paid.headers.get(name));
  console.log(`  paid ${paid.status} settle.success=${settle?.success} tx=${settle?.transaction}`);
  results.push({
    sku: sku.name,
    ok: paid.status === 200 && settle?.success !== false,
    status: paid.status,
    settle,
    bodyPreview: typeof body === "string" ? body : JSON.stringify(body).slice(0, 180),
  });
}

console.log("\n== summary ==");
console.log(JSON.stringify(results, null, 2));
const failed = results.filter((r) => !r.ok);
process.exit(failed.length ? 1 : 0);
