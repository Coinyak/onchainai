// Shared agent-surface heuristics for operator demote/promote scripts.
// Positive match: tool IS an agent/MCP/skill surface (keep official).
// Aligns with src/crawler/sources/vendor_orgs.rs has_agent_surface keywords
// plus catalog taxonomy (type/function/actor).

/** Slugs never demoted by demote-non-agent-official.mjs */
export const DEMOTE_ALLOWLIST = [
  // fixtures/discovery-ground-truth.json
  "tiny-place",
  "x402",
  "clawrouter",
  "agentkit",
  "agenti",
  "goldrush-x402",
  "onchainai",
  "aifinpay-agent",

  // scripts/seed-platform-agent-tools.mjs
  "github-mcp-server",
  "x-api-mcp",
  "x-api-typescript-sdk",
  "neynar-nodejs-sdk",
  "farcaster-hub-nodejs",
  "discord-interactions-js",
  "telegram-bot-api-server",
  "grammy-telegram-bot",

  // scripts/seed-vendor-agent-surfaces.mjs
  "solana-mcp-official",
  "payments-mcp",
  "agentic-wallet-skills",
  "chainlink-agent-skills",
  "circlefin-skills",
  "base-skills",
  "metamask-skills",
  "opensea-skill",
  "crossmint-agentic-finance",

  // post-bulk official agent surfaces
  "x402-chat",
  "blockrun-mcp",
  "polymarket-agent",
  "openhuman",
  "clawrouter-hermes",
  "awesome-finance-mcp",
  "awesome-openclaw-money-maker",
  "solana-dev-skill",
  "ton-triage-skill",
  "walletconnect-skills",
  "pay-skills",
  "tinyagents",

  // MCP SDK — agent surface, not infra library
  "modelcontextprotocol-sdk",
];

/** SQL boolean — true when row matches agent surface (Postgres). */
export const AGENT_SURFACE_SQL = `
  (
    type IN ('mcp', 'skill', 'x402', 'cli')
    OR function IN ('ai-agent', 'payments', 'wallet', 'identity', 'social', 'oracle')
    OR actor = 'ai-agent'
    OR lower(slug) ~ '(^|[^a-z])(agent|mcp|skill|x402|clawrouter|openclaw)([^a-z]|$)'
    OR lower(name) ~ '(^|[^a-z])(agent|mcp|skill|x402|clawrouter|openclaw)([^a-z]|$)'
    OR slug IN (
      'tiny-place', 'x402', 'clawrouter', 'agentkit', 'agenti', 'goldrush-x402',
      'onchainai', 'aifinpay-agent'
    )
    OR (source IN ('manual', 'self') AND type = 'mcp')
  )
`;

export function allowlistLower() {
  return DEMOTE_ALLOWLIST.map((s) => s.toLowerCase());
}