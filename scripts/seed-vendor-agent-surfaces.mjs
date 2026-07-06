#!/usr/bin/env node
// Operator gate fix for first-party vendor_orgs agent/MCP/skill surfaces.
// Sets approval_status + relevance_status only — never mutates tools.status on conflict.
// Gate fixes skip quarantined/rejected rows unless SEED_FORCE_GATE_FIX=1.
//
// Usage:
//   node scripts/seed-vendor-agent-surfaces.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-vendor-agent-surfaces.mjs

import {
  tool,
  loadEnv,
  connectPg,
  runInTransaction,
  forceGateFix,
} from "./seed-tool-lib.mjs";

const TOOLS = [
  tool({
    slug: "solana-mcp-official",
    name: "Solana MCP (Official)",
    description:
      "Official Solana Foundation MCP server for AI agents — wallet balances, transactions, and onchain program queries on Solana.",
    function: "data",
    type: "mcp",
    repo_url: "https://github.com/solana-foundation/solana-mcp-official",
    homepage: "https://github.com/solana-foundation/solana-mcp-official",
    chains: ["solana"],
    source: "vendor_orgs",
    stars: 79,
    crypto_relevance_score: 88,
    crypto_relevance_reasons: [
      "mentions Solana",
      "wallet/custody tooling",
      "operator-curated: Solana Foundation official MCP",
    ],
  }),
  tool({
    slug: "payments-mcp",
    name: "Coinbase Payments MCP",
    description:
      "Coinbase Payments MCP — agent-installable MCP surface for x402 and onchain payment flows on Base.",
    function: "payments",
    type: "mcp",
    repo_url: "https://github.com/coinbase/payments-mcp",
    homepage: "https://github.com/coinbase/payments-mcp",
    chains: ["base"],
    source: "vendor_orgs",
    stars: 56,
    crypto_relevance_score: 86,
    crypto_relevance_reasons: [
      "x402 payments",
      "mentions Base",
      "operator-curated: Coinbase vendor MCP",
    ],
  }),
  tool({
    slug: "agentic-wallet-skills",
    name: "Coinbase Agentic Wallet Skills",
    description:
      "Official Coinbase agentic wallet skills for OpenClaw — onchain wallet actions and agent treasury workflows.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/coinbase/agentic-wallet-skills",
    homepage: "https://github.com/coinbase/agentic-wallet-skills",
    chains: ["base", "ethereum"],
    source: "vendor_orgs",
    stars: 120,
    crypto_relevance_score: 86,
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "mentions Base",
      "operator-curated: Coinbase agent skills",
    ],
  }),
  tool({
    slug: "chainlink-agent-skills",
    name: "Chainlink Agent Skills",
    description:
      "Official Chainlink agent skills implementing agentskills.io — oracle, CCIP, and automation surfaces for AI agents.",
    function: "oracle",
    type: "sdk",
    repo_url: "https://github.com/smartcontractkit/chainlink-agent-skills",
    homepage: "https://github.com/smartcontractkit/chainlink-agent-skills",
    chains: ["ethereum"],
    source: "vendor_orgs",
    stars: 117,
    crypto_relevance_score: 84,
    crypto_relevance_reasons: [
      "oracle/price feeds",
      "operator-curated: Chainlink official agent skills",
    ],
  }),
  tool({
    slug: "circlefin-skills",
    name: "Circle Agent Skills",
    description:
      "Circle open-source agent skills for USDC treasury, payments, and developer workflows.",
    function: "payments",
    type: "sdk",
    repo_url: "https://github.com/circlefin/skills",
    homepage: "https://github.com/circlefin/skills",
    chains: ["ethereum", "base"],
    source: "vendor_orgs",
    stars: 126,
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "stablecoin/payments",
      "operator-curated: Circle official skills",
    ],
  }),
  tool({
    slug: "base-skills",
    name: "Base Agent Skills",
    description:
      "Official Base (Coinbase L2) agent skills for onchain development and agent workflows.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/base/skills",
    homepage: "https://github.com/base/skills",
    chains: ["base"],
    source: "vendor_orgs",
    stars: 89,
    crypto_relevance_score: 84,
    crypto_relevance_reasons: [
      "mentions Base",
      "operator-curated: Base official skills",
    ],
  }),
  tool({
    slug: "metamask-skills",
    name: "MetaMask Agent Skills",
    description:
      "MetaMask ecosystem agent skills for OpenClaw — wallet, swaps, and dapp connectivity for autonomous agents.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/MetaMask/skills",
    homepage: "https://github.com/MetaMask/skills",
    chains: ["ethereum"],
    source: "vendor_orgs",
    stars: 19,
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "operator-curated: MetaMask official skills",
    ],
  }),
  tool({
    slug: "opensea-skill",
    name: "OpenSea Agent Skill",
    description:
      "Official OpenSea agent skill for NFT discovery, listings, and marketplace actions.",
    function: "nft",
    type: "sdk",
    repo_url: "https://github.com/ProjectOpenSea/opensea-skill",
    homepage: "https://github.com/ProjectOpenSea/opensea-skill",
    chains: ["ethereum"],
    source: "vendor_orgs",
    stars: 43,
    crypto_relevance_score: 80,
    crypto_relevance_reasons: [
      "NFT marketplace",
      "operator-curated: OpenSea official skill",
    ],
  }),
  tool({
    slug: "crossmint-agentic-finance",
    name: "Crossmint Agentic Finance",
    description:
      "Crossmint starter code and demos for agentic finance — wallets, checkout, and treasury for AI agents.",
    function: "payments",
    type: "sdk",
    repo_url: "https://github.com/Crossmint/crossmint-agentic-finance",
    homepage: "https://github.com/Crossmint/crossmint-agentic-finance",
    chains: ["ethereum", "solana"],
    source: "vendor_orgs",
    stars: 19,
    crypto_relevance_score: 84,
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "operator-curated: Crossmint agent finance",
    ],
  }),
];

const UPSERT_SQL = `
INSERT INTO tools (
  name, slug, description, function, asset_class, actor, type,
  repo_url, homepage, npm_package, install_command, mcp_endpoint,
  chains, approval_status, rejection_reason,
  crypto_relevance_score, crypto_relevance_reasons, relevance_status,
  install_risk_level, install_risk_reasons, requires_secret,
  license, pricing, stars, source, review_policy_version,
  created_at, updated_at
) VALUES (
  $1, $2, $3, $4, $5, $6, $7,
  $8, $9, $10, $11, $12,
  $13, 'approved', NULL,
  $14, $15, 'accepted',
  $16, $17, $18,
  $19, 'free', $20, $21, 'operator-vendor-agent-surfaces-v1',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  function = EXCLUDED.function,
  asset_class = EXCLUDED.asset_class,
  actor = EXCLUDED.actor,
  type = EXCLUDED.type,
  repo_url = COALESCE(EXCLUDED.repo_url, tools.repo_url),
  homepage = COALESCE(EXCLUDED.homepage, tools.homepage),
  npm_package = COALESCE(EXCLUDED.npm_package, tools.npm_package),
  install_command = COALESCE(EXCLUDED.install_command, tools.install_command),
  mcp_endpoint = COALESCE(EXCLUDED.mcp_endpoint, tools.mcp_endpoint),
  chains = CASE WHEN cardinality(EXCLUDED.chains) > 0 THEN EXCLUDED.chains ELSE tools.chains END,
  approval_status = CASE
    WHEN $22::boolean THEN 'approved'
    WHEN tools.quarantined_at IS NOT NULL THEN tools.approval_status
    WHEN tools.approval_status = 'rejected' THEN tools.approval_status
    ELSE 'approved'
  END,
  rejection_reason = CASE
    WHEN $22::boolean THEN NULL
    WHEN tools.quarantined_at IS NOT NULL OR tools.approval_status = 'rejected' THEN tools.rejection_reason
    ELSE NULL
  END,
  crypto_relevance_score = EXCLUDED.crypto_relevance_score,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = CASE
    WHEN $22::boolean THEN 'accepted'
    WHEN tools.quarantined_at IS NOT NULL THEN tools.relevance_status
    WHEN tools.relevance_status = 'rejected' THEN tools.relevance_status
    ELSE 'accepted'
  END,
  install_risk_level = EXCLUDED.install_risk_level,
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  requires_secret = EXCLUDED.requires_secret,
  license = COALESCE(EXCLUDED.license, tools.license),
  stars = GREATEST(tools.stars, EXCLUDED.stars),
  source = EXCLUDED.source,
  review_policy_version = EXCLUDED.review_policy_version,
  quarantined_at = CASE
    WHEN $22::boolean THEN NULL
    ELSE tools.quarantined_at
  END,
  updated_at = now()
RETURNING slug, approval_status, relevance_status, (xmax = 0) AS inserted;
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
          script: "seed-vendor-agent-surfaces.mjs",
          tool_count: TOOLS.length,
          slugs: TOOLS.map((t) => t.slug),
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-vendor-agent-surfaces.mjs",
          force_gate_fix_hint:
            "Add SEED_FORCE_GATE_FIX=1 to override quarantined/rejected rows",
        },
        null,
        2,
      ),
    );
    return;
  }

  const client = await connectPg(env);
  const force = forceGateFix(env);
  try {
    const results = await runInTransaction(client, async () => {
      const rows = [];
      for (const t of TOOLS) {
        const r = await client.query(UPSERT_SQL, [
          t.name,
          t.slug,
          t.description,
          t.function,
          t.asset_class,
          t.actor,
          t.type,
          t.repo_url,
          t.homepage,
          t.npm_package,
          t.install_command,
          t.mcp_endpoint,
          t.chains,
          t.crypto_relevance_score,
          t.crypto_relevance_reasons,
          t.install_risk_level,
          t.install_risk_reasons,
          t.requires_secret,
          t.license,
          t.stars,
          t.source,
          force,
        ]);
        rows.push({
          slug: r.rows[0].slug,
          action: r.rows[0].inserted ? "inserted" : "updated",
          approval_status: r.rows[0].approval_status,
          relevance_status: r.rows[0].relevance_status,
        });
      }
      return rows;
    });
    console.log(
      JSON.stringify(
        {
          ok: true,
          mode: "apply",
          script: "seed-vendor-agent-surfaces.mjs",
          force_gate_fix: force,
          tools: results,
        },
        null,
        2,
      ),
    );
  } catch (error) {
    console.error(error);
    process.exit(1);
  } finally {
    await client.end();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});