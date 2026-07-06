#!/usr/bin/env node
// Operator metadata enrichment for 12 verified-candidate tools.
// Aligns homepage (and repo where needed) so verify-tool-official.mjs identity
// cluster or domain-verified official paths can succeed. UPDATE-only — never inserts rows.
//
// Usage:
//   node scripts/seed-verified-candidates.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-verified-candidates.mjs

import { tool, loadEnv, connectPg, runInTransaction } from "./seed-tool-lib.mjs";

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
    homepage: "https://gelato.cloud",
    npm_package: "@gelatodigital/gasless",
    crypto_relevance_reasons: [
      "automation/relay",
      "operator-curated: gelatodigital org + scoped npm + gelato.cloud identity cluster",
    ],
  }),
  tool({
    slug: "socket-v2-sdk",
    homepage: "https://socket.tech",
    npm_package: "@socketdottech/socket-v2-sdk",
    crypto_relevance_reasons: [
      "bridge/cross-chain",
      "operator-curated: SocketDotTech org + scoped npm + socket.tech identity cluster",
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
    npm_package: "@envio-dev/hypersync-client",
    crypto_relevance_reasons: [
      "indexer/data",
      "operator-curated: enviodev org + @envio-dev scope + envio.dev identity cluster",
    ],
  }),
  tool({
    slug: "moralis-web3-sdk",
    homepage: "https://moralis.io",
    npm_package: "@moralisweb3/common-evm-utils",
    crypto_relevance_reasons: [
      "Web3 API",
      "operator-curated: MoralisWeb3 org + @moralisweb3 scope + moralis.io identity cluster",
    ],
  }),
  tool({
    slug: "chaingate",
    homepage: "https://chaingate.dev",
    npm_package: "@drakensoftware/chaingate",
    crypto_relevance_reasons: [
      "wallet/dev tooling",
      "operator-curated: drakensoftware scoped npm (homepage cluster pending)",
    ],
  }),
  tool({
    slug: "chaingate-react",
    homepage: "https://chaingate.dev",
    npm_package: "@drakensoftware/chaingate-react",
    crypto_relevance_reasons: [
      "wallet/dev tooling",
      "operator-curated: drakensoftware scoped npm (homepage cluster pending)",
    ],
  }),
];

const UPDATE_SQL = `
UPDATE tools SET
  repo_url = COALESCE($1, repo_url),
  homepage = COALESCE($2, homepage),
  npm_package = COALESCE($3, npm_package),
  install_command = COALESCE($4, install_command),
  mcp_endpoint = COALESCE($5, mcp_endpoint),
  crypto_relevance_reasons = CASE
    WHEN $6::text[] IS NOT NULL AND cardinality($6::text[]) > 0 THEN $6
    ELSE crypto_relevance_reasons
  END,
  review_policy_version = 'operator-verified-candidates-v1',
  updated_at = now()
WHERE slug = $7
RETURNING slug, homepage, repo_url;
`;

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

  const client = await connectPg(env);
  try {
    const results = await runInTransaction(client, async () => {
      const existing = await client.query(
        `SELECT slug FROM tools WHERE slug = ANY($1::text[])`,
        [TOOLS.map((t) => t.slug)],
      );
      const found = new Set(existing.rows.map((r) => r.slug));
      const missing = TOOLS.filter((t) => !found.has(t.slug)).map((t) => t.slug);
      if (missing.length) {
        throw new Error(
          `missing slugs in DB (update-only seed): ${missing.join(", ")}`,
        );
      }

      const rows = [];
      for (const patch of TOOLS) {
        const r = await client.query(UPDATE_SQL, [
          patch.repo_url ?? null,
          patch.homepage ?? null,
          patch.npm_package ?? null,
          patch.install_command ?? null,
          patch.mcp_endpoint ?? null,
          patch.crypto_relevance_reasons ?? null,
          patch.slug,
        ]);
        if (r.rowCount !== 1) {
          throw new Error(`update failed for slug ${patch.slug}`);
        }
        rows.push(r.rows[0]);
      }
      return rows;
    });
    console.log(JSON.stringify({ ok: true, mode: "apply", tools: results }, null, 2));
  } finally {
    await client.end();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});