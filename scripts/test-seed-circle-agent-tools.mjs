#!/usr/bin/env node
// Structural test for PR-1 Circle for Agents seed manifest.
// Drives seed dry-run and exercises verify-tool-official FIRST_PARTY_ORGS loader.
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const EXPECTED_SLUGS = [
  "circle-agent-stack",
  "circle-x402-batching",
  "circle-gateway",
  "circle-cctp-v2",
  "circle-cctp-provider-sdk",
  "circle-dev-controlled-wallets",
  "circle-user-controlled-wallets",
  "circle-modular-wallets",
  "circle-paymaster",
  "circle-api-node-sdk",
  "usdc-stablecoin-contracts",
];

function fail(msg) {
  console.error(`SEED CIRCLE TEST FAIL: ${msg}`);
  process.exit(1);
}

const firstPartyOrgs = loadFirstPartyOrgs();
if (firstPartyOrgs.circlefin !== "Circle") {
  fail(`FIRST_PARTY_ORGS loader missing circlefin: "Circle" (got ${firstPartyOrgs.circlefin ?? "undefined"})`);
}

if (EXPECTED_SLUGS.includes("skills")) {
  fail("skills slug must not be in Circle seed manifest (circlefin/skills collision guard)");
}

const dryRun = execFileSync("node", ["scripts/seed-circle-agent-tools.mjs"], {
  cwd: ROOT,
  encoding: "utf8",
});
const payload = JSON.parse(dryRun.trim());
if (payload.mode !== "dry-run") fail(`expected dry-run mode, got ${payload.mode}`);
if (payload.tool_count !== 11) fail(`expected tool_count 11, got ${payload.tool_count}`);

const slugs = new Set(payload.slugs ?? []);
for (const slug of EXPECTED_SLUGS) {
  if (!slugs.has(slug)) fail(`missing slug in dry-run output: ${slug}`);
}
if (slugs.size !== 11) fail(`expected exactly 11 slugs, got ${slugs.size}`);

console.log(
  JSON.stringify({
    ok: true,
    test: "seed-circle-agent-tools",
    tool_count: payload.tool_count,
    first_party_org: "circlefin",
  }),
);