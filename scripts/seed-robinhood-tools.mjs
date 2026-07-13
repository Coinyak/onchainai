#!/usr/bin/env node
// Robinhood Chain + Robinhood brokerage agent surfaces (external discovery).
//
//   node scripts/seed-robinhood-tools.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-robinhood-tools.mjs
//
// Sources: docs.robinhood.com/chain, Alchemy/Across RH support announcements,
// GitHub (nhevers/project-r0x, trayders/trayd-mcp, verygoodplugins/robinhood-mcp, …).

import { tool, runSeed } from "./seed-tool-lib.mjs";

const TOOLS = [
  // --- Robinhood Chain (eip155:4663) ---
  tool({
    slug: "r0x-os",
    name: "r0x (Robinhood Chain x402)",
    description:
      "USDG-native x402 facilitator + SDK for Robinhood Chain — agents discover priced skills, settle micropayments, trade, bridge, and call MCP tools without API keys.",
    function: "payments",
    type: "sdk",
    actor: "ai-agent",
    repo_url: "https://github.com/nhevers/project-r0x",
    homepage: "https://projectr0x.dev",
    npm_package: "r0x-os",
    install_command: "npm i r0x-os",
    mcp_endpoint: null,
    chains: ["robinhood"],
    stars: 123,
    license: "MIT",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: [
      "wallet private key required for USDG x402 settlement",
      "documented npm package + Claude Code plugin",
    ],
    crypto_relevance_score: 95,
    crypto_relevance_reasons: [
      "official Robinhood Chain x402 facilitator (projectr0x.dev)",
      "npm r0x-os SDK + MCP skill catalog on eip155:4663",
      "GitHub topics: robinhood-chain, x402, mcp, payments",
    ],
    source: "operator-curated",
  }),
  tool({
    slug: "alchemy-sdk",
    name: "Alchemy SDK",
    description:
      "Alchemy Web3 development platform — RPC, NFT API, webhooks, and account abstraction helpers (includes Robinhood Chain).",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/alchemyplatform/alchemy-sdk-js",
    homepage: "https://www.alchemy.com/docs/reference/robinhood-chain-api-quickstart",
    npm_package: "alchemy-sdk",
    install_command: "npm i alchemy-sdk",
    chains: ["ethereum", "arbitrum", "monad", "polygon", "base", "robinhood"],
    crypto_relevance_score: 90,
    crypto_relevance_reasons: [
      "Alchemy documents Robinhood Chain mainnet RPC + tooling",
      "recommended infra provider in docs.robinhood.com/chain",
    ],
    source: "operator-curated",
  }),
  tool({
    slug: "across-protocol-sdk",
    name: "Across Protocol SDK",
    description:
      "Intent-based cross-chain transfers — quote routes, build deposits, track fills across L2s including Robinhood Chain.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/across-protocol/sdk",
    homepage: "https://across.to/blog/bridge-to-robinhood-chain-with-across",
    npm_package: "@across-protocol/sdk",
    install_command: "npm i @across-protocol/sdk",
    chains: ["arbitrum", "base", "optimism", "ethereum", "robinhood"],
    stars: 28,
    crypto_relevance_score: 88,
    crypto_relevance_reasons: [
      "Across announced day-one Robinhood Chain bridging (USDC/USDG)",
      "intent bridge SDK for L2 deposits and fills",
    ],
    source: "operator-curated",
  }),

  // --- Robinhood brokerage / TradFi agent MCP (not L2; no chain tag) ---
  tool({
    slug: "trayd-mcp",
    name: "Trayd MCP",
    description:
      "Remote MCP for Robinhood brokerage — portfolio, quotes, buy/sell/short/ladder orders via Claude Code or claude.ai connectors.",
    function: "trading",
    type: "mcp",
    actor: "ai-agent",
    asset_class: "crypto",
    repo_url: "https://github.com/trayders/trayd-mcp",
    homepage: "https://github.com/trayders/trayd-mcp",
    mcp_endpoint: "https://mcp.trayd.ai/mcp",
    install_command: "claude mcp add --transport http trayd https://mcp.trayd.ai/mcp --scope user",
    chains: [],
    stars: 34,
    requires_secret: true,
    install_risk_level: "high",
    install_risk_reasons: [
      "live order execution against a real brokerage account",
      "OAuth/account linking required",
    ],
    crypto_relevance_score: 72,
    crypto_relevance_reasons: [
      "agent-native MCP for Robinhood trading (TradFi crossover)",
      "remote HTTP MCP endpoint documented for Claude Code",
    ],
    source: "operator-curated",
  }),
  tool({
    slug: "robinhood-mcp",
    name: "Robinhood MCP (read-only)",
    description:
      "Read-only MCP for Robinhood portfolio research via robin_stocks — positions, dividends, watchlists; no trade execution.",
    function: "data",
    type: "mcp",
    actor: "ai-agent",
    repo_url: "https://github.com/verygoodplugins/robinhood-mcp",
    homepage: "https://pypi.org/project/robinhood-mcp/",
    install_command: "pip install robinhood-mcp",
    chains: [],
    stars: 33,
    license: "MIT",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: [
      "Robinhood credentials via unofficial API (robin_stocks)",
      "read-only by design — no trade tools",
    ],
    crypto_relevance_score: 70,
    crypto_relevance_reasons: [
      "MCP portfolio research for Robinhood accounts",
      "PyPI package robinhood-mcp",
    ],
    source: "operator-curated",
  }),
  tool({
    slug: "robinhood-crypto-mcp-server",
    name: "Robinhood Crypto MCP Server",
    description:
      "MCP server for the Robinhood Crypto API — auth, account, market data, and crypto trading over REST and WebSocket.",
    function: "trading",
    type: "mcp",
    actor: "ai-agent",
    repo_url: "https://github.com/rohitsingh-iitd/robinhood-mcp-server",
    homepage: "https://github.com/rohitsingh-iitd/robinhood-mcp-server",
    install_command: "pip install -r requirements.txt",
    chains: [],
    stars: 30,
    requires_secret: true,
    install_risk_level: "high",
    install_risk_reasons: [
      "ROBINHOOD_API_KEY and private key required",
      "supports crypto trading operations",
    ],
    crypto_relevance_score: 82,
    crypto_relevance_reasons: [
      "Robinhood Crypto API surface for agents",
      "GitHub topics: crypto, mcp-server, robinhood-api",
    ],
    source: "operator-curated",
  }),
  tool({
    slug: "open-stocks-mcp",
    name: "Open Stocks MCP",
    description:
      "Multi-broker MCP for stocks and options — Robinhood + Schwab tools, HTTP/stdio transport, Docker-friendly.",
    function: "trading",
    type: "mcp",
    actor: "ai-agent",
    repo_url: "https://github.com/Open-Agent-Tools/open-stocks-mcp",
    homepage: "https://github.com/Open-Agent-Tools/open-stocks-mcp",
    install_command: "pip install open-stocks-mcp",
    chains: [],
    stars: 9,
    license: "Apache-2.0",
    requires_secret: true,
    install_risk_level: "high",
    install_risk_reasons: [
      "live stock/options trading validated against real accounts",
      "broker credentials in environment",
    ],
    crypto_relevance_score: 68,
    crypto_relevance_reasons: [
      "agent MCP with Robinhood broker support",
      "multi-broker architecture for trading agents",
    ],
    source: "operator-curated",
  }),
];

await runSeed(TOOLS, "seed-robinhood-tools.mjs");
