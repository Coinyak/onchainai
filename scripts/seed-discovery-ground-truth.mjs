#!/usr/bin/env node
// Operator-curated upsert for discovery ground-truth tools blocked on public gates
// (approval_status / relevance_status). Fixes ClawHub/vendor crawls that lack
// repo/homepage evidence and were auto-rejected by the relevance scanner.
//
// New rows use DB default status (community). ON CONFLICT never mutates tools.status.
// Gate fixes skip quarantined/rejected rows unless SEED_FORCE_GATE_FIX=1.
//
// Usage:
//   node scripts/seed-discovery-ground-truth.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-discovery-ground-truth.mjs

import {
  tool,
  loadEnv,
  connectPg,
  runInTransaction,
  forceGateFix,
} from "./seed-tool-lib.mjs";

const TOOLS = [
  tool({
    slug: "tiny-place",
    name: "tiny.place",
    description:
      "Agent-to-agent social network on Solana with x402 payments. Onboard a @handle identity, run check-in loops for DMs, feed, and bounties via the tinyplace CLI (@tinyhumansai/tinyplace).",
    function: "social",
    type: "skill",
    actor: "ai-agent",
    repo_url: "https://github.com/tinyhumansai/tiny.place",
    homepage: "https://tiny.place",
    npm_package: "@tinyhumansai/tinyplace",
    install_command: "clawhub install tinyplace",
    chains: ["solana"],
    source: "clawhub",
    stars: 0,
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "x402 payments",
      "mentions Solana",
      "wallet/custody tooling",
      "operator-curated: ClawHub skill with GitHub + npm evidence",
      "discovery-ground-truth fixture",
    ],
    install_risk_level: "medium",
    install_risk_reasons: ["documented clawhub + npm CLI install"],
  }),
  tool({
    slug: "agentkit",
    name: "AgentKit",
    description:
      "Official Coinbase AgentKit — wallet and onchain action SDK for autonomous AI agents on Base and Ethereum.",
    function: "wallet",
    type: "sdk",
    actor: "ai-agent",
    repo_url: "https://github.com/coinbase/agentkit",
    homepage: "https://github.com/coinbase/agentkit",
    chains: ["base", "ethereum"],
    source: "vendor_orgs",
    stars: 0,
    crypto_relevance_score: 86,
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "mentions Base",
      "mentions Ethereum",
      "operator-curated: Coinbase vendor org ground-truth",
      "discovery-ground-truth fixture",
    ],
    install_risk_level: "medium",
    install_risk_reasons: ["official Coinbase SDK — review install docs before production"],
  }),
  tool({
    slug: "x402",
    name: "X402",
    description:
      "Search and call paid API services using the x402 payment protocol (HTTP 402). Discover bazaar endpoints, browse payment requirements, and make x402-paid requests from agent skills.",
    function: "payments",
    type: "skill",
    actor: "ai-agent",
    repo_url: "https://github.com/x402-foundation/x402",
    homepage: "https://x402.org",
    install_command: "clawhub install x402-2",
    chains: ["base", "solana"],
    source: "clawhub",
    stars: 0,
    crypto_relevance_score: 88,
    crypto_relevance_reasons: [
      "x402 payments",
      "operator-curated: x402 Foundation reference + ClawHub skill",
      "discovery-ground-truth fixture",
    ],
    install_risk_level: "medium",
    install_risk_reasons: ["documented clawhub skill install"],
  }),
  tool({
    slug: "clawrouter",
    name: "ClawRouter",
    description:
      "Agent-native LLM router for OpenClaw — 41+ models, sub-millisecond routing, USDC payments on Base and Solana via x402.",
    function: "payments",
    type: "mcp",
    actor: "ai-agent",
    repo_url: "https://github.com/BlockRunAI/ClawRouter",
    homepage: "https://github.com/BlockRunAI/ClawRouter",
    chains: ["base", "solana"],
    source: "vendor_orgs",
    stars: 0,
    crypto_relevance_score: 90,
    crypto_relevance_reasons: [
      "x402 payments",
      "mentions Base",
      "mentions Solana",
      "operator-curated: BlockRun vendor org ground-truth",
      "discovery-ground-truth fixture",
    ],
    install_risk_level: "medium",
    install_risk_reasons: ["review upstream install docs before production"],
  }),
  tool({
    slug: "goldrush-x402",
    name: "Goldrush X402",
    description:
      "GoldRush x402 — pay-per-request blockchain data access using the x402 protocol (HTTP 402 Payment Required) for AI agent workflows.",
    function: "data",
    type: "skill",
    actor: "ai-agent",
    repo_url: "https://github.com/covalenthq/goldrush-mcp-server",
    homepage: "https://www.covalenthq.com",
    npm_package: "@covalenthq/client-sdk",
    install_command: "clawhub install goldrush-x402",
    chains: ["ethereum"],
    source: "clawhub",
    stars: 0,
    crypto_relevance_score: 78,
    crypto_relevance_reasons: [
      "x402 payments",
      "blockchain keyword",
      "operator-curated: regression ClawHub x402 skill",
      "discovery-ground-truth fixture",
    ],
    install_risk_level: "medium",
    install_risk_reasons: ["documented clawhub skill install"],
  }),
  tool({
    slug: "aifinpay-agent",
    name: "@aifinpay/agent",
    description:
      "Unified agent-economy SDK for autonomous AI agents — chain-opaque AiFinPayAgent surface over Polygon and Solana.",
    function: "payments",
    type: "sdk",
    actor: "ai-agent",
    repo_url: "https://github.com/AiFinPay/sdk",
    homepage: "https://aifinpay.com",
    npm_package: "@aifinpay/agent",
    install_command: "npm i @aifinpay/agent",
    chains: ["solana", "polygon"],
    source: "npm",
    stars: 0,
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "wallet/custody tooling",
      "mentions Solana",
      "operator-curated: npm agent SDK identity cluster",
    ],
    install_risk_level: "low",
    install_risk_reasons: ["documented npm install"],
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
  $19, 'free', $20, $21, 'operator-discovery-ground-truth-v1',
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
          script: "seed-discovery-ground-truth.mjs",
          tool_count: TOOLS.length,
          slugs: TOOLS.map((t) => t.slug),
          gates_fixed: ["approval_status=approved", "relevance_status=accepted"],
          apply_hint:
            "ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-discovery-ground-truth.mjs",
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
          script: "seed-discovery-ground-truth.mjs",
          force_gate_fix: force,
          tools: results,
        },
        null,
        2,
      ),
    );
  } finally {
    await client.end();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});