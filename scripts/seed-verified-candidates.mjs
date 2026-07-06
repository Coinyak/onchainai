#!/usr/bin/env node
// Operator metadata enrichment for 12 verified-candidate tools.
// Aligns homepage (and repo where needed) so verify-tool-official.mjs identity
// cluster or domain-verified official paths can succeed. Does NOT set tools.status.
//
// Usage:
//   node scripts/seed-verified-candidates.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-verified-candidates.mjs

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { tool, loadEnv } from "./seed-tool-lib.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

const TOOLS = [
  tool({
    slug: "rainbowkit",
    homepage: "https://www.rainbow.me",
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "operator-curated: identity cluster homepage aligned to rainbow-me org",
    ],
  }),
  tool({
    slug: "across-protocol-sdk",
    homepage: "https://across.to",
    crypto_relevance_reasons: [
      "bridge/cross-chain",
      "operator-curated: identity cluster homepage aligned to across-protocol org",
    ],
  }),
  tool({
    slug: "coingecko-mcp",
    homepage: "https://www.coingecko.com",
    mcp_endpoint: "https://mcp.api.coingecko.com/mcp",
    crypto_relevance_reasons: [
      "market data",
      "operator-curated: identity cluster homepage aligned to coingecko org",
    ],
  }),
  tool({
    slug: "privy-node-sdk",
    homepage: "https://privy.io",
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "operator-curated: identity cluster homepage aligned to privy-io org",
    ],
  }),
  tool({
    slug: "subsquid-cli",
    homepage: "https://subsquid.dev",
    crypto_relevance_reasons: [
      "indexer/data",
      "operator-curated: identity cluster homepage aligned to subsquid org",
    ],
  }),
  tool({
    slug: "gelato-gasless-sdk",
    homepage: "https://www.gelato.cloud",
    crypto_relevance_reasons: [
      "automation/relay",
      "operator-curated: org site alignment for Gelato gasless SDK",
    ],
  }),
  tool({
    slug: "socket-v2-sdk",
    homepage: "https://socket.tech",
    crypto_relevance_reasons: [
      "bridge/cross-chain",
      "operator-curated: org site alignment for Socket V2 SDK",
    ],
  }),
  tool({
    slug: "mantle-sdk",
    repo_url: "https://github.com/mantlenetworkio/mantle",
    npm_package: "@mantlenetworkio/sdk",
    install_command: "npm i @mantlenetworkio/sdk",
    homepage: "https://mantle.xyz",
    crypto_relevance_reasons: [
      "L2 dev SDK",
      "operator-curated: @mantlenetworkio/sdk identity cluster aligned to Mantle org",
    ],
  }),
  tool({
    slug: "envio-hyperindex",
    homepage: "https://envio.dev",
    crypto_relevance_reasons: [
      "indexer/data",
      "operator-curated: domain alignment for Envio HyperIndex",
    ],
  }),
  tool({
    slug: "moralis-web3-sdk",
    homepage: "https://moralis.io",
    crypto_relevance_reasons: [
      "Web3 API",
      "operator-curated: domain alignment for Moralis SDK",
    ],
  }),
  tool({
    slug: "chaingate",
    homepage: "https://chaingate.dev",
    crypto_relevance_reasons: [
      "wallet/dev tooling",
      "operator-curated: product homepage for chaingate npm package",
    ],
  }),
  tool({
    slug: "chaingate-react",
    homepage: "https://chaingate.dev",
    crypto_relevance_reasons: [
      "wallet/dev tooling",
      "operator-curated: product homepage for chaingate-react npm package",
    ],
  }),
];

const UPSERT_SQL = `
INSERT INTO tools (
  name, slug, description, function, asset_class, actor, type,
  repo_url, homepage, npm_package, install_command, mcp_endpoint,
  chains, status, approval_status, rejection_reason,
  crypto_relevance_score, crypto_relevance_reasons, relevance_status,
  install_risk_level, install_risk_reasons, requires_secret,
  license, pricing, stars, source, review_policy_version,
  created_at, updated_at
) VALUES (
  $1, $2, $3, $4, $5, $6, $7,
  $8, $9, $10, $11, $12,
  $13, 'community', 'approved', NULL,
  $14, $15, 'accepted',
  $16, $17, $18,
  $19, 'free', $20, $21, 'operator-verified-candidates-v1',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  repo_url = COALESCE(EXCLUDED.repo_url, tools.repo_url),
  homepage = COALESCE(EXCLUDED.homepage, tools.homepage),
  npm_package = COALESCE(EXCLUDED.npm_package, tools.npm_package),
  install_command = COALESCE(EXCLUDED.install_command, tools.install_command),
  mcp_endpoint = COALESCE(EXCLUDED.mcp_endpoint, tools.mcp_endpoint),
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  review_policy_version = EXCLUDED.review_policy_version,
  updated_at = now()
RETURNING slug, homepage, repo_url, (xmax = 0) AS inserted;
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
  if (!apply) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "dry-run",
          script: "seed-verified-candidates.mjs",
          slugs: TOOLS.map((t) => t.slug),
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-verified-candidates.mjs",
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

  const existing = await client.query(
    `SELECT slug, name, description, function, asset_class, actor, type,
            repo_url, homepage, npm_package, install_command, mcp_endpoint,
            chains, crypto_relevance_score, install_risk_level, install_risk_reasons,
            requires_secret, license, stars, source
     FROM tools WHERE slug = ANY($1::text[])`,
    [TOOLS.map((t) => t.slug)],
  );
  const bySlug = Object.fromEntries(existing.rows.map((r) => [r.slug, r]));
  const missing = TOOLS.filter((t) => !bySlug[t.slug]).map((t) => t.slug);
  if (missing.length) {
    console.error(`missing slugs in DB: ${missing.join(", ")}`);
    process.exit(2);
  }

  const results = [];
  for (const patch of TOOLS) {
    const base = bySlug[patch.slug];
    const merged = { ...base, ...patch };
    const r = await client.query(UPSERT_SQL, [
      merged.name,
      merged.slug,
      merged.description,
      merged.function,
      merged.asset_class,
      merged.actor,
      merged.type,
      merged.repo_url,
      merged.homepage,
      merged.npm_package,
      merged.install_command,
      merged.mcp_endpoint,
      merged.chains,
      merged.crypto_relevance_score,
      merged.crypto_relevance_reasons,
      merged.install_risk_level,
      merged.install_risk_reasons,
      merged.requires_secret,
      merged.license,
      merged.stars,
      merged.source,
    ]);
    results.push(r.rows[0]);
  }
  await client.end();
  console.log(JSON.stringify({ ok: true, mode: "apply", tools: results }, null, 2));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});