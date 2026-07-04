#!/usr/bin/env node
/**
 * E2E: OnchainAI K2 premium check_endpoint_health x402 payment.
 *
 * Usage:
 *   EVM_PRIVATE_KEY=0x... node scripts/x402-premium-e2e.mjs [slug] [api_base]
 *
 * Defaults:
 *   slug: goldrush-x402
 *   api_base: https://onchainai-production.up.railway.app
 */
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { registerExactEvmScheme } from "@x402/evm/exact/client";
import { privateKeyToAccount } from "viem/accounts";

const slug = process.argv[2] || "goldrush-x402";
const apiBase = (process.argv[3] || "https://onchainai-production.up.railway.app").replace(
  /\/$/,
  "",
);
const url = `${apiBase}/api/v2/premium/check-endpoint-health/${slug}`;

const privateKey = process.env.EVM_PRIVATE_KEY;
if (!privateKey || !/^0x[0-9a-fA-F]{64}$/.test(privateKey)) {
  console.error(
    "Set EVM_PRIVATE_KEY (0x + 64 hex) — Base mainnet wallet with USDC + ETH for gas.",
  );
  process.exit(1);
}

const signer = privateKeyToAccount(privateKey);
const client = new x402Client();
registerExactEvmScheme(client, { signer });
const fetchWithPayment = wrapFetchWithPayment(fetch, client);

console.log(`[1/3] Unpaid probe: ${url}`);
const unpaid = await fetch(url, { method: "GET" });
console.log(`  status=${unpaid.status}`);
const paymentRequired = unpaid.headers.get("payment-required");
if (unpaid.status !== 402 || !paymentRequired) {
  console.error("Expected HTTP 402 with PAYMENT-REQUIRED header");
  process.exit(1);
}
console.log("  PAYMENT-REQUIRED: ok");

console.log(`[2/3] Paid request (wallet ${signer.address})`);
const paid = await fetchWithPayment(url, { method: "GET" });
const bodyText = await paid.text();
console.log(`  status=${paid.status}`);

let body;
try {
  body = JSON.parse(bodyText);
} catch {
  body = bodyText;
}

const httpClient = new x402HTTPClient(client);
const settle = httpClient.getPaymentSettleResponse((name) => paid.headers.get(name));

console.log("[3/3] Result");
console.log(JSON.stringify({ status: paid.status, settle, body }, null, 2));

if (paid.status !== 200) {
  process.exit(1);
}