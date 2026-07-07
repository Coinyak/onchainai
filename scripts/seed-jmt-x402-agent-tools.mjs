#!/usr/bin/env node
// Upsert JMT x402 Agent Tools into the catalog — 25 paid x402 endpoints on
// Base mainnet (web search, AI analysis, crypto/stock data, SEC filings,
// company intel, news, sentiment, macro dashboard). Local LLM-powered.
// $0.001–$0.15/call USDC over the x402 protocol.
//
// Listing status (official/verified) is not set here — use
// verify-tool-official.mjs for that, per repo convention.
//
// Usage:
//   node scripts/seed-jmt-x402-agent-tools.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 \
//     node scripts/seed-jmt-x402-agent-tools.mjs

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadEnv } from "./seed-tool-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

const X402_ENDPOINT = "https://jmt-x402-proxy.jmthomasofficial.workers.dev";
const HOMEPAGE = X402_ENDPOINT;
const REPO = "https://github.com/jmthomasofficial";

const UPSERT_SQL = `
INSERT INTO tools (
  name, slug, description, function, asset_class, actor, type,
  repo_url, homepage, npm_package, install_command, mcp_endpoint,
  chains, status, source, approval_status, rejection_reason,
  crypto_relevance_score, crypto_relevance_reasons, relevance_status,
  install_risk_level, install_risk_reasons, requires_secret,
  license, pricing, x402_price, x402_endpoint, stars,
  review_policy_version,
  created_at, updated_at
) VALUES (
  $1, $2, $3, $4, 'crypto', 'ai-agent', 'x402',
  $5, $6, NULL, NULL, NULL,
  $7, 'community', 'manual', 'approved', NULL,
  $8, $9, 'accepted',
  'low', $10, false,
  NULL, 'x402', $11, $12, 0,
  'operator-aggregator-curate-v1',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  function = EXCLUDED.function,
  repo_url = EXCLUDED.repo_url,
  homepage = EXCLUDED.homepage,
  chains = EXCLUDED.chains,
  crypto_relevance_score = EXCLUDED.crypto_relevance_score,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = 'accepted',
  install_risk_level = 'low',
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  pricing = 'x402',
  x402_price = EXCLUDED.x402_price,
  x402_endpoint = EXCLUDED.x402_endpoint,
  updated_at = now()
RETURNING slug, (xmax = 0) AS inserted;
`;

function pgSslOption(env, databaseUrl) {
  const mode = (env.PGSSLMODE || "").toLowerCase();
  const wantsSsl =
    mode === "require" ||
    /supabase\.(co|com)/i.test(databaseUrl) ||
    databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  return true;
}

async function main() {
  const env = loadEnv();
  const apply = env.SEED_ENV === "prod-curate";
  const tool = {
    name: "JMT x402 Agent Tools",
    slug: "jmt-x402-agent-tools",
    description:
      "25 paid x402 endpoints on Base mainnet — web search, AI analysis, crypto and stock market data, SEC filings, company intel, news, sentiment, and a macro dashboard. Local LLM-powered. Pay-per-call over the x402 protocol ($0.001–$0.15/call USDC).",
    function: "ai-agent",
    repo_url: REPO,
    homepage: HOMEPAGE,
    chains: ["base"],
    crypto_relevance_reasons: [
      "x402 paid endpoints on Base mainnet (USDC settlement)",
      "crypto + stock market data, SEC filings, onchain-relevant intel",
      "agent-native pay-per-call API surface (HTTP 402 handshake)",
    ],
    install_risk_reasons: [
      "HTTP 402 API surface — no local install or secret required",
    ],
    x402_price: "$0.001–$0.15/call USDC on Base mainnet",
    x402_endpoint: X402_ENDPOINT,
  };

  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "dry-run",
          tool,
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-jmt-x402-agent-tools.mjs",
        },
        null,
        2,
      ),
    );
    return;
  }

  const DATABASE_URL = env.DATABASE_URL || "";
  if (!DATABASE_URL) {
    console.error("DATABASE_URL missing");
    process.exit(2);
  }

  const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
  const ssl = pgSslOption(env, DATABASE_URL);
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(ssl !== undefined ? { ssl } : {}),
  });
  await client.connect();
  const r = await client.query(UPSERT_SQL, [
    tool.name,
    tool.slug,
    tool.description,
    tool.function,
    tool.repo_url,
    tool.homepage,
    tool.chains,
    82,
    tool.crypto_relevance_reasons,
    tool.install_risk_reasons,
    tool.x402_price,
    tool.x402_endpoint,
  ]);
  await client.end();
  console.log(
    JSON.stringify(
      {
        ok: true,
        mode: "apply",
        slug: r.rows[0].slug,
        action: r.rows[0].inserted ? "inserted" : "updated",
      },
      null,
      2,
    ),
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});