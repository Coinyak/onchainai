#!/usr/bin/env node
// Operator-curated first-party / high-trust agent tools for GitHub, X, Farcaster,
// Discord, and Telegram. Idempotent upsert by slug. Prod-safe: read-only gate on
// SEED_ENV=prod-curate (explicit operator action).
//
// Usage:
//   ENV_FILE=/path/to/.env SEED_ENV=prod-curate node scripts/seed-platform-agent-tools.mjs
//   ENV_FILE=/path/to/.env SEED_ENV=prod-curate FORCE_APPROVE=1 node scripts/seed-platform-agent-tools.mjs

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

function parseEnvFile(path) {
  const out = {};
  try {
    const text = readFileSync(path, "utf8");
    for (const raw of text.split("\n")) {
      const line = raw.trim();
      if (!line || line.startsWith("#")) continue;
      const eq = line.indexOf("=");
      if (eq <= 0) continue;
      const key = line.slice(0, eq).trim();
      let value = line.slice(eq + 1);
      const hash = value.search(/\s+#/);
      if (hash >= 0) value = value.slice(0, hash);
      value = value.trim().replace(/^["']|["']$/g, "");
      if (key) out[key] = value;
    }
  } catch {
    /* optional */
  }
  return out;
}

const env = {
  ...parseEnvFile(process.env.ENV_FILE || resolve(ROOT, ".env")),
  ...process.env,
};

if (env.SEED_ENV !== "prod-curate") {
  console.error(
    "refusing: set SEED_ENV=prod-curate to upsert operator-curated platform tools",
  );
  process.exit(2);
}

const DATABASE_URL = env.DATABASE_URL || "";
if (!DATABASE_URL) {
  console.error("DATABASE_URL missing");
  process.exit(2);
}

/** @type {import('pg').Client} */
const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));

const TOOLS = [
  {
    slug: "github-mcp-server",
    name: "GitHub MCP Server",
    description:
      "GitHub's official MCP server for AI agents — browse crypto and web3 repos, triage issues and PRs, monitor Actions CI/CD pipelines, and analyze smart-contract codebases.",
    function: "dev-tool",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "mcp",
    repo_url: "https://github.com/github/github-mcp-server",
    homepage: "https://github.com/github/github-mcp-server",
    npm_package: null,
    install_command: "npx mcp-remote https://api.githubcopilot.com/mcp/",
    mcp_endpoint: "https://api.githubcopilot.com/mcp/",
    chains: [],
    stars: 31154,
    license: "MIT",
    source: "manual",
    crypto_relevance_score: 78,
    crypto_relevance_reasons: [
      "operator-curated: agents audit, build, and deploy onchain protocol repos via GitHub",
      "smart contract tooling",
      "onchain keyword",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: true,
  },
  {
    slug: "x-api-mcp",
    name: "X API MCP Server",
    description:
      "Official FastMCP server from X Developer Platform exposing the X API as MCP tools — crypto project announcements, community monitoring, and agent social workflows.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "mcp",
    repo_url: "https://github.com/xdevplatform/xmcp",
    homepage: "https://developer.x.com",
    npm_package: null,
    install_command:
      "git clone https://github.com/xdevplatform/xmcp && cd xmcp && pip install -r requirements.txt",
    mcp_endpoint: null,
    chains: [],
    stars: 828,
    license: null,
    source: "manual",
    crypto_relevance_score: 72,
    crypto_relevance_reasons: [
      "operator-curated: onchain project announcements and crypto community ops on X",
      "onchain keyword",
      "agent surface for crypto social stacks",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "medium",
    install_risk_reasons: ["requires cloning repo and pip install"],
    requires_secret: true,
  },
  {
    slug: "x-api-typescript-sdk",
    name: "X API TypeScript SDK",
    description:
      "Official TypeScript SDK for the X API (v2) — build crypto trading bots, onchain project alert agents, and social automation for web3 communities.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "sdk",
    repo_url: "https://github.com/xdevplatform/twitter-api-typescript-sdk",
    homepage: "https://developer.x.com",
    npm_package: "twitter-api-sdk",
    install_command: "npm i twitter-api-sdk",
    mcp_endpoint: null,
    chains: [],
    stars: 991,
    license: "Apache-2.0",
    source: "manual",
    crypto_relevance_score: 74,
    crypto_relevance_reasons: [
      "operator-curated: official X API SDK for crypto alert bots and web3 social agents",
      "onchain keyword",
      "token tooling",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: true,
  },
  {
    slug: "neynar-nodejs-sdk",
    name: "Neynar Node.js SDK",
    description:
      "Official TypeScript SDK for Neynar Farcaster APIs — read feeds, post casts, manage channels, and power wallet-linked social agents on Base.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "sdk",
    repo_url: "https://github.com/neynarxyz/nodejs-sdk",
    homepage: "https://docs.neynar.com",
    npm_package: "@neynar/nodejs-sdk",
    install_command: "npm i @neynar/nodejs-sdk",
    mcp_endpoint: null,
    chains: ["base"],
    stars: 70,
    license: "MIT",
    source: "manual",
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "operator-curated: Farcaster onchain social protocol SDK with wallet-linked identity",
      "mentions Base",
      "onchain keyword",
      "supports chain: base",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: true,
  },
  {
    slug: "farcaster-hub-nodejs",
    name: "Farcaster Hub Node.js",
    description:
      "Lightweight TypeScript client for Farcaster Hubs — sync casts, verify onchain social messages, and build decentralized agent integrations on the Farcaster protocol.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "sdk",
    repo_url: "https://github.com/farcasterxyz/hub-monorepo",
    homepage: "https://www.farcaster.xyz",
    npm_package: "@farcaster/hub-nodejs",
    install_command: "npm i @farcaster/hub-nodejs",
    mcp_endpoint: null,
    chains: ["base"],
    stars: 827,
    license: "MIT",
    source: "manual",
    crypto_relevance_score: 85,
    crypto_relevance_reasons: [
      "operator-curated: Farcaster decentralized onchain social protocol hub client",
      "mentions Base",
      "onchain keyword",
      "supports chain: base",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: false,
  },
  {
    slug: "discord-interactions-js",
    name: "Discord Interactions.js",
    description:
      "Official Discord helpers for slash commands and webhooks — foundation for crypto trading alert bots, DAO governance bots, and community agent stacks.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "sdk",
    repo_url: "https://github.com/discord/discord-interactions-js",
    homepage: "https://discord.com/developers/docs/interactions/overview",
    npm_package: "discord-interactions",
    install_command: "npm i discord-interactions",
    mcp_endpoint: null,
    chains: [],
    stars: 511,
    license: "MIT",
    source: "manual",
    crypto_relevance_score: 71,
    crypto_relevance_reasons: [
      "operator-curated: official Discord bot SDK for crypto trading alerts and DAO community agents",
      "onchain keyword",
      "governance tooling for DAO Discord bots",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: true,
  },
  {
    slug: "telegram-bot-api-server",
    name: "Telegram Bot API Server",
    description:
      "Official Telegram Bot API server (TDLib) — self-hosted backend for crypto wallet alert bots, trading signal agents, and Telegram automation in web3 stacks.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "api",
    repo_url: "https://github.com/tdlib/telegram-bot-api",
    homepage: "https://core.telegram.org/bots/api",
    npm_package: null,
    install_command: null,
    mcp_endpoint: null,
    chains: [],
    stars: 4316,
    license: "BSL-1.0",
    source: "manual",
    crypto_relevance_score: 73,
    crypto_relevance_reasons: [
      "operator-curated: official Telegram Bot API for crypto wallet alerts and trading agents",
      "wallet/custody tooling",
      "onchain keyword",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "medium",
    install_risk_reasons: ["no install command provided", "requires compiling C++ server"],
    requires_secret: true,
  },
  {
    slug: "grammy-telegram-bot",
    name: "grammY Telegram Bot Framework",
    description:
      "High-trust TypeScript Telegram bot framework widely used in crypto trading bots, wallet notification agents, and DeFi alert pipelines.",
    function: "social",
    asset_class: "crypto",
    actor: "ai-agent",
    type: "sdk",
    repo_url: "https://github.com/grammyjs/grammY",
    homepage: "https://grammy.dev",
    npm_package: "grammy",
    install_command: "npm i grammy",
    mcp_endpoint: null,
    chains: [],
    stars: 3659,
    license: "MIT",
    source: "manual",
    crypto_relevance_score: 76,
    crypto_relevance_reasons: [
      "operator-curated: de-facto Telegram SDK in crypto trading and wallet alert agent stacks",
      "DeFi keyword",
      "wallet/custody tooling",
      "onchain keyword",
      "has trustworthy listing evidence",
    ],
    relevance_status: "accepted",
    install_risk_level: "low",
    install_risk_reasons: ["documented package manager install"],
    requires_secret: true,
  },
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
  $14, $15, $16,
  $17, $18, $19,
  $20, 'free', $21, $22, 'operator-platform-curate-v1',
  now(), now()
)
ON CONFLICT (slug) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  function = EXCLUDED.function,
  asset_class = EXCLUDED.asset_class,
  actor = EXCLUDED.actor,
  type = EXCLUDED.type,
  repo_url = EXCLUDED.repo_url,
  homepage = EXCLUDED.homepage,
  npm_package = EXCLUDED.npm_package,
  install_command = EXCLUDED.install_command,
  mcp_endpoint = EXCLUDED.mcp_endpoint,
  chains = EXCLUDED.chains,
  approval_status = CASE
    WHEN $23::boolean THEN 'approved'
    ELSE tools.approval_status
  END,
  rejection_reason = CASE
    WHEN $23::boolean THEN NULL
    ELSE tools.rejection_reason
  END,
  crypto_relevance_score = EXCLUDED.crypto_relevance_score,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = CASE
    WHEN $23::boolean THEN EXCLUDED.relevance_status
    WHEN tools.relevance_status = 'rejected' THEN tools.relevance_status
    ELSE EXCLUDED.relevance_status
  END,
  install_risk_level = EXCLUDED.install_risk_level,
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  requires_secret = EXCLUDED.requires_secret,
  license = EXCLUDED.license,
  stars = GREATEST(tools.stars, EXCLUDED.stars),
  source = EXCLUDED.source,
  review_policy_version = EXCLUDED.review_policy_version,
  quarantined_at = CASE
    WHEN $23::boolean THEN NULL
    ELSE tools.quarantined_at
  END,
  status = CASE
    WHEN $23::boolean THEN EXCLUDED.status
    WHEN tools.status IN ('official', 'verified') THEN tools.status
    ELSE EXCLUDED.status
  END,
  updated_at = now()
RETURNING slug, (xmax = 0) AS inserted;
`;

const FORCE_APPROVE = env.FORCE_APPROVE === "1";

const client = new pg.Client({
  connectionString: DATABASE_URL,
  ssl: { rejectUnauthorized: false },
});

await client.connect();
const results = [];
for (const tool of TOOLS) {
  const r = await client.query(UPSERT_SQL, [
    tool.name,
    tool.slug,
    tool.description,
    tool.function,
    tool.asset_class,
    tool.actor,
    tool.type,
    tool.repo_url,
    tool.homepage,
    tool.npm_package,
    tool.install_command,
    tool.mcp_endpoint,
    tool.chains,
    tool.crypto_relevance_score,
    tool.crypto_relevance_reasons,
    tool.relevance_status,
    tool.install_risk_level,
    tool.install_risk_reasons,
    tool.requires_secret,
    tool.license,
    tool.stars,
    tool.source,
    FORCE_APPROVE,
  ]);
  results.push({
    slug: tool.slug,
    action: r.rows[0].inserted ? "inserted" : "updated",
  });
}
await client.end();

console.log(JSON.stringify({ ok: true, tools: results }, null, 2));