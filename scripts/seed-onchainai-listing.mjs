#!/usr/bin/env node
// Upsert OnchainAI as first-party tool in the catalog (MCP + x402 discovery).
// Listing status (official/verified) must be set via verify-tool-official.mjs only.
//
// Usage:
//   node scripts/seed-onchainai-listing.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-onchainai-listing.mjs

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { loadEnv } from "./seed-tool-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

const MCP_ENDPOINT = "https://www.onchain-ai.xyz/mcp";
const HOMEPAGE = "https://www.onchain-ai.xyz";
const REPO = "https://github.com/Coinyak/onchainai";
const INSTALL =
  "claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp";

const UPSERT_SQL = `
INSERT INTO tools (
  name, slug, description, function, asset_class, actor, type,
  repo_url, homepage, npm_package, install_command, mcp_endpoint,
  chains, status, official_team, source, approval_status, rejection_reason,
  crypto_relevance_score, crypto_relevance_reasons, relevance_status,
  install_risk_level, install_risk_reasons, requires_secret,
  license, pricing, x402_price, stars, logo_url,
  referral_enabled, x402_builder_code,
  created_at, updated_at
) VALUES (
  $1, $2, $3, 'dev-tool', 'crypto', 'ai-agent', 'mcp',
  $4, $5, NULL, $6, $7,
  $8, 'community', 'OnchainAI', 'self', 'approved', NULL,
  100, $9, 'accepted',
  'low', $10, false,
  'MIT', 'x402', $11, 0, $12,
  false, 'bc_ljttbnhv',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  repo_url = EXCLUDED.repo_url,
  homepage = EXCLUDED.homepage,
  install_command = EXCLUDED.install_command,
  mcp_endpoint = EXCLUDED.mcp_endpoint,
  chains = EXCLUDED.chains,
  official_team = 'OnchainAI',
  source = 'self',
  approval_status = 'approved',
  crypto_relevance_score = 100,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = 'accepted',
  install_risk_level = 'low',
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  license = EXCLUDED.license,
  pricing = 'x402',
  x402_price = EXCLUDED.x402_price,
  x402_builder_code = EXCLUDED.x402_builder_code,
  logo_url = EXCLUDED.logo_url,
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
    name: "OnchainAI",
    slug: "onchainai",
    description:
      "Crypto tool directory for AI agents — discover, vet, and install MCP servers, CLIs, SDKs, APIs, and x402 services. Free MCP discovery; Agent Trust micro-payments optional before third-party x402 calls.",
    repo_url: REPO,
    homepage: HOMEPAGE,
    install_command: INSTALL,
    mcp_endpoint: MCP_ENDPOINT,
    chains: ["base", "ethereum", "solana"],
    crypto_relevance_reasons: [
      "first-party official directory",
      "mcp streamable-http endpoint",
      "x402 discovery and trust metadata",
    ],
    install_risk_reasons: ["official HTTP MCP — no local install required"],
    x402_price:
      "Discovery free; check_endpoint_health $0.001/call (Agent Trust, OnchainAI payee)",
    logo_url: "/brand/onchainai-logo.png",
  };

  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "dry-run",
          tool,
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-onchainai-listing.mjs",
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
  const client = new pg.Client({
    connectionString: DATABASE_URL,
    ...(pgSslOption(env, DATABASE_URL) !== undefined
      ? { ssl: pgSslOption(env, DATABASE_URL) }
      : {}),
  });
  await client.connect();
  const r = await client.query(UPSERT_SQL, [
    tool.name,
    tool.slug,
    tool.description,
    tool.repo_url,
    tool.homepage,
    tool.install_command,
    tool.mcp_endpoint,
    tool.chains,
    tool.crypto_relevance_reasons,
    tool.install_risk_reasons,
    tool.x402_price,
    tool.logo_url,
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