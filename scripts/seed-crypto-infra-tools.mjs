#!/usr/bin/env node
// Operator-curated crypto infra tools discovered via DeFiLlama, CoinGecko, and
// official chain ecosystem docs (aggregators = discovery index; evidence = homepage/repo).
//
//   node scripts/seed-crypto-infra-tools.mjs              # dry-run count
//   ENV_FILE=.env SEED_ENV=prod-curate node scripts/seed-crypto-infra-tools.mjs

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

function parseEnvFile(path) {
  const out = {};
  try {
    for (const raw of readFileSync(path, "utf8").split("\n")) {
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
const APPLY = env.SEED_ENV === "prod-curate";
const DRY = !APPLY;

/** @param {Partial<ToolRow> & Pick<ToolRow, "slug"|"name"|"description"|"function"|"type"|"homepage">} row */
function tool(row) {
  return {
    asset_class: "crypto",
    actor: row.actor ?? "ai-agent",
    repo_url: row.repo_url ?? null,
    npm_package: row.npm_package ?? null,
    install_command: row.install_command ?? null,
    mcp_endpoint: row.mcp_endpoint ?? null,
    chains: row.chains ?? [],
    stars: row.stars ?? 0,
    license: row.license ?? null,
    source: "aggregator",
    crypto_relevance_score: row.crypto_relevance_score ?? 80,
    crypto_relevance_reasons: row.crypto_relevance_reasons ?? [
      "aggregator-discovery: DeFiLlama/CoinGecko/official ecosystem evidence",
      "operator-curated infra tool for onchain agents",
    ],
    relevance_status: "accepted",
    install_risk_level: row.install_risk_level ?? "low",
    install_risk_reasons: row.install_risk_reasons ?? [
      row.install_command ? "documented package manager install" : "HTTP API surface",
    ],
    requires_secret: row.requires_secret ?? false,
    ...row,
  };
}

const TOOLS = [
  // --- Official chain MCPs ---
  tool({
    slug: "base-mcp",
    name: "Base MCP",
    description:
      "Official Base wallet MCP — balances, sends, swaps, signing, and x402 payments via Base Account OAuth.",
    function: "wallet",
    type: "mcp",
    homepage: "https://docs.base.org/agents",
    install_command: "npx mcp-remote https://mcp.base.org",
    mcp_endpoint: "https://mcp.base.org",
    chains: ["base"],
    requires_secret: true,
    crypto_relevance_score: 92,
  }),
  tool({
    slug: "base-docs-mcp",
    name: "Base Docs MCP",
    description: "Official Base documentation MCP for live docs search — OnchainKit, smart contracts, and agent guides.",
    function: "dev-tool",
    type: "mcp",
    homepage: "https://docs.base.org/get-started/docs-mcp",
    install_command: "npx mcp-remote https://docs.base.org/mcp",
    mcp_endpoint: "https://docs.base.org/mcp",
    chains: ["base"],
    crypto_relevance_score: 85,
  }),
  tool({
    slug: "optimism-docs-mcp",
    name: "Optimism Docs MCP",
    description: "Official Optimism docs MCP — OP Stack bridging, interop, fees, and chain deployment.",
    function: "dev-tool",
    type: "mcp",
    homepage: "https://docs.optimism.io",
    install_command: "npx mcp-remote https://docs.optimism.io/mcp",
    mcp_endpoint: "https://docs.optimism.io/mcp",
    chains: ["optimism"],
    crypto_relevance_score: 82,
  }),
  tool({
    slug: "polygon-docs-mcp",
    name: "Polygon Docs MCP",
    description: "Official Polygon Developer Docs MCP — CDK, Agglayer, and Open Money Stack.",
    function: "dev-tool",
    type: "mcp",
    homepage: "https://docs.polygon.technology",
    install_command: "npx mcp-remote https://docs.polygon.technology/mcp",
    mcp_endpoint: "https://docs.polygon.technology/mcp",
    chains: ["polygon"],
    crypto_relevance_score: 82,
  }),
  tool({
    slug: "coingecko-mcp",
    name: "CoinGecko MCP",
    description: "Official CoinGecko MCP — live prices, market data, DeFi pools, and onchain analytics (free keyless tier).",
    function: "data",
    type: "mcp",
    repo_url: "https://github.com/coingecko/coingecko-typescript",
    homepage: "https://mcp.api.coingecko.com/",
    npm_package: "@coingecko/coingecko-mcp",
    install_command: "npx mcp-remote https://mcp.api.coingecko.com/mcp",
    mcp_endpoint: "https://mcp.api.coingecko.com/mcp",
    chains: [],
    crypto_relevance_score: 88,
  }),
  // --- Base / L2 dev SDKs ---
  tool({
    slug: "onchainkit",
    name: "OnchainKit",
    description: "Coinbase/Base React/TS SDK — wallet, transactions, identity, and MiniKit for onchain apps.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/coinbase/onchainkit",
    homepage: "https://onchainkit.xyz",
    npm_package: "@coinbase/onchainkit",
    install_command: "npm i @coinbase/onchainkit",
    chains: ["base", "ethereum"],
    stars: 1200,
    license: "MIT",
    crypto_relevance_score: 90,
  }),
  tool({
    slug: "mantle-sdk",
    name: "Mantle SDK",
    description: "Official @mantleio/sdk for L1↔L2 deposits, withdrawals, and cross-domain messaging on Mantle.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/mantlenetworkio/mantle",
    homepage: "https://sdk.mantle.xyz",
    npm_package: "@mantleio/sdk",
    install_command: "npm i @mantleio/sdk",
    chains: ["mantle"],
    crypto_relevance_score: 84,
  }),
  // --- Bridge / cross-chain SDKs ---
  tool({
    slug: "layerzero-devtools-sdk",
    name: "LayerZero DevTools",
    description: "Official LayerZero tooling for omnichain OApps/OFTs and cross-chain messaging on 150+ chains.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/LayerZero-Labs/devtools",
    homepage: "https://docs.layerzero.network",
    npm_package: "@layerzerolabs/toolbox-hardhat",
    install_command: "npm i @layerzerolabs/toolbox-hardhat",
    chains: ["ethereum", "base", "arbitrum", "optimism", "polygon"],
    stars: 215,
    crypto_relevance_score: 90,
  }),
  tool({
    slug: "wormhole-typescript-sdk",
    name: "Wormhole TypeScript SDK",
    description: "Official Wormhole SDK for cross-chain transfers and generic messaging (Portal Bridge).",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/wormhole-foundation/wormhole-sdk-ts",
    homepage: "https://wormhole.com",
    npm_package: "@wormhole-foundation/sdk",
    install_command: "npm i @wormhole-foundation/sdk",
    chains: ["ethereum", "solana", "base", "arbitrum"],
    stars: 81,
    license: "Apache-2.0",
    crypto_relevance_score: 88,
  }),
  tool({
    slug: "hyperlane-sdk",
    name: "Hyperlane SDK",
    description: "Permissionless interchain messaging — deploy mailboxes, configure ISMs, route cross-chain messages.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/hyperlane-xyz/hyperlane-monorepo",
    homepage: "https://www.hyperlane.xyz",
    npm_package: "@hyperlane-xyz/sdk",
    install_command: "npm i @hyperlane-xyz/sdk",
    chains: ["ethereum", "base", "arbitrum", "optimism"],
    stars: 64,
    license: "Apache-2.0",
    crypto_relevance_score: 86,
  }),
  tool({
    slug: "axelar-js-sdk",
    name: "Axelar JavaScript SDK",
    description: "Axelar GMP SDK for cross-chain token transfers and gateway contract interactions.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/axelarnetwork/axelarjs-sdk",
    homepage: "https://axelar.network",
    npm_package: "@axelar-network/axelarjs-sdk",
    install_command: "npm i @axelar-network/axelarjs-sdk",
    chains: ["ethereum", "base", "arbitrum", "polygon"],
    stars: 35,
    license: "MIT",
    crypto_relevance_score: 85,
  }),
  tool({
    slug: "chainlink-ccip-sdk",
    name: "Chainlink CCIP SDK",
    description: "Official Chainlink CCIP TypeScript SDK for cross-chain token transfers and arbitrary messaging.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/smartcontractkit/ccip-tools-ts",
    homepage: "https://docs.chain.link/ccip",
    npm_package: "@chainlink/ccip-sdk",
    install_command: "npm i @chainlink/ccip-sdk",
    chains: ["ethereum", "base", "arbitrum", "optimism"],
    stars: 16,
    license: "MIT",
    crypto_relevance_score: 87,
  }),
  tool({
    slug: "across-protocol-sdk",
    name: "Across Protocol SDK",
    description: "Intent-based cross-chain transfers — quote routes, build deposits, track fills across L2s.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/across-protocol/sdk",
    homepage: "https://docs.across.to",
    npm_package: "@across-protocol/sdk",
    install_command: "npm i @across-protocol/sdk",
    chains: ["ethereum", "base", "arbitrum", "optimism"],
    stars: 28,
    crypto_relevance_score: 83,
  }),
  tool({
    slug: "socket-v2-sdk",
    name: "Socket V2 SDK",
    description: "Socket.tech SDK for cross-chain bridging and liquidity routing in dApps and agents.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/SocketDotTech/socket-v2-sdk",
    homepage: "https://www.socket.tech",
    npm_package: "@socket.tech/socket-v2-sdk",
    install_command: "npm i @socket.tech/socket-v2-sdk",
    chains: ["ethereum", "base", "arbitrum", "polygon"],
    crypto_relevance_score: 80,
  }),
  tool({
    slug: "lifi-sdk",
    name: "LI.FI SDK",
    description: "Any-to-any cross-chain swap SDK — aggregate bridges and DEXs for agent routing and execution.",
    function: "bridge",
    type: "sdk",
    repo_url: "https://github.com/lifinance/sdk",
    homepage: "https://li.fi",
    npm_package: "@lifi/sdk",
    install_command: "npm i @lifi/sdk",
    chains: ["ethereum", "base", "arbitrum", "solana"],
    stars: 260,
    license: "Apache-2.0",
    crypto_relevance_score: 89,
  }),
  tool({
    slug: "lifi-api",
    name: "LI.FI REST API",
    description: "LI.FI public REST API for cross-chain quotes and routes at li.quest/v1 (agent-friendly HTTP).",
    function: "bridge",
    type: "api",
    repo_url: "https://github.com/lifinance/sdk",
    homepage: "https://docs.li.fi/api-reference/introduction",
    chains: ["ethereum", "base", "arbitrum", "solana"],
    actor: "ai-agent",
    requires_secret: true,
    crypto_relevance_score: 86,
  }),
  // --- Oracles / data ---
  tool({
    slug: "chainlink-sdk",
    name: "Chainlink",
    description: "Chainlink oracle network — price feeds, CCIP, Automation; smartcontractkit/chainlink reference implementation.",
    function: "data",
    type: "sdk",
    repo_url: "https://github.com/smartcontractkit/chainlink",
    homepage: "https://chain.link/",
    chains: ["ethereum", "base", "arbitrum", "polygon"],
    stars: 8000,
    crypto_relevance_score: 91,
  }),
  tool({
    slug: "pyth-network-sdk",
    name: "Pyth Network",
    description: "Low-latency oracle network — Hermes API and client SDKs for real-time price feeds.",
    function: "data",
    type: "sdk",
    repo_url: "https://github.com/pyth-network/pyth-crosschain",
    homepage: "https://pyth.network/",
    chains: ["ethereum", "solana", "base", "arbitrum"],
    crypto_relevance_score: 90,
  }),
  tool({
    slug: "redstone-oracles-sdk",
    name: "RedStone Oracles",
    description: "Modular oracle delivering price feeds for DeFi, RWAs, and agent workflows.",
    function: "data",
    type: "sdk",
    repo_url: "https://github.com/redstone-finance",
    homepage: "https://www.redstone.finance/",
    chains: ["ethereum", "arbitrum", "base"],
    crypto_relevance_score: 86,
  }),
  tool({
    slug: "api3-airnode",
    name: "Api3 Airnode",
    description: "First-party oracle API — connect web APIs to smart contracts without intermediaries.",
    function: "data",
    type: "api",
    repo_url: "https://github.com/api3dao/airnode",
    homepage: "https://api3.org/",
    chains: ["ethereum", "polygon"],
    crypto_relevance_score: 84,
  }),
  tool({
    slug: "the-graph-cli",
    name: "The Graph CLI",
    description: "Deploy and manage subgraphs — decentralized indexing for onchain data and agent queries.",
    function: "data",
    type: "cli",
    repo_url: "https://github.com/graphprotocol/graph-tooling",
    homepage: "https://thegraph.com/docs",
    npm_package: "@graphprotocol/graph-cli",
    install_command: "npm i -g @graphprotocol/graph-cli",
    chains: ["ethereum", "arbitrum", "polygon"],
    crypto_relevance_score: 88,
  }),
  tool({
    slug: "band-protocol-sdk",
    name: "Band Protocol",
    description: "Cross-chain oracle SDK for custom data feeds and price references.",
    function: "data",
    type: "sdk",
    repo_url: "https://github.com/bandprotocol/",
    homepage: "https://bandprotocol.com/",
    chains: ["ethereum", "cosmos"],
    crypto_relevance_score: 82,
  }),
  // --- RPC / infra ---
  tool({
    slug: "alchemy-sdk",
    name: "Alchemy SDK",
    description: "Alchemy Web3 development platform — RPC, NFT API, webhooks, and account abstraction helpers.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/alchemyplatform/alchemy-sdk-js",
    homepage: "https://www.alchemy.com",
    npm_package: "alchemy-sdk",
    install_command: "npm i alchemy-sdk",
    chains: ["ethereum", "base", "arbitrum", "polygon", "monad"],
    requires_secret: true,
    crypto_relevance_score: 87,
  }),
  tool({
    slug: "quicknode-sdk",
    name: "QuickNode SDK",
    description: "Multi-chain RPC and developer APIs with typed SDK for agent backends.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/quicknode/sdk",
    homepage: "https://www.quicknode.com",
    npm_package: "@quicknode/sdk",
    install_command: "npm i @quicknode/sdk",
    chains: ["ethereum", "solana", "base", "monad"],
    requires_secret: true,
    crypto_relevance_score: 85,
  }),
  tool({
    slug: "ankr-advanced-api",
    name: "Ankr Advanced API",
    description: "Ankr multichain RPC and Advanced API — balances, NFTs, and token price queries.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/Ankr-network/ankr.js",
    homepage: "https://www.ankr.com/rpc",
    npm_package: "@ankr.com/ankr.js",
    install_command: "npm i @ankr.com/ankr.js",
    chains: ["ethereum", "polygon", "avalanche"],
    requires_secret: true,
    crypto_relevance_score: 83,
  }),
  tool({
    slug: "moralis-web3-sdk",
    name: "Moralis Web3 SDK",
    description: "Moralis unified Web3 API — wallet history, token prices, NFT metadata for agent backends.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/MoralisWeb3/Moralis-JS-SDK",
    homepage: "https://docs.moralis.com",
    npm_package: "moralis",
    install_command: "npm i moralis",
    chains: ["ethereum", "polygon", "bsc"],
    requires_secret: true,
    crypto_relevance_score: 84,
  }),
  tool({
    slug: "pocket-network-sdk",
    name: "Pocket Network SDK",
    description: "Decentralized RPC via Pocket Network — pocketjs provider for multichain agents.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/pokt-network/pocket-js",
    homepage: "https://docs.pokt.network",
    npm_package: "@pokt-foundation/pocketjs-provider",
    install_command: "npm i @pokt-foundation/pocketjs-provider",
    chains: ["ethereum", "polygon"],
    crypto_relevance_score: 81,
  }),
  tool({
    slug: "etherscan-api",
    name: "Etherscan API",
    description: "Etherscan multichain explorer API — contracts, transactions, and verification for agent due diligence.",
    function: "data",
    type: "api",
    repo_url: "https://github.com/etherscan/awesome-etherscan",
    homepage: "https://docs.etherscan.io",
    chains: ["ethereum", "base", "arbitrum"],
    requires_secret: true,
    crypto_relevance_score: 86,
  }),
  tool({
    slug: "dune-analytics-api",
    name: "Dune Analytics API",
    description: "Dune query API client — run SQL analytics and fetch results for onchain intelligence agents.",
    function: "data",
    type: "api",
    repo_url: "https://github.com/duneanalytics/ts-dune-client",
    homepage: "https://dune.com",
    npm_package: "@duneanalytics/client-sdk",
    install_command: "npm i @duneanalytics/client-sdk",
    chains: ["ethereum", "polygon", "base"],
    requires_secret: true,
    crypto_relevance_score: 87,
  }),
  // --- Indexers ---
  tool({
    slug: "goldsky-cli",
    name: "Goldsky CLI",
    description: "Goldsky subgraph and Mirror pipelines — real-time indexed onchain data for agents.",
    function: "data",
    type: "cli",
    repo_url: "https://github.com/goldsky-io/streamling",
    homepage: "https://docs.goldsky.com",
    npm_package: "@goldskycom/cli",
    install_command: "npm i -g @goldskycom/cli",
    chains: ["ethereum", "base", "arbitrum"],
    requires_secret: true,
    crypto_relevance_score: 84,
  }),
  tool({
    slug: "envio-hyperindex",
    name: "Envio HyperIndex",
    description: "Envio indexer framework with HyperSync — GraphQL outputs for multichain agent data layers.",
    function: "data",
    type: "cli",
    repo_url: "https://github.com/enviodev/hyperindex",
    homepage: "https://envio.dev",
    npm_package: "envio",
    install_command: "npm i envio",
    chains: ["ethereum", "monad", "base"],
    crypto_relevance_score: 83,
  }),
  tool({
    slug: "subsquid-cli",
    name: "Subsquid CLI",
    description: "Subsquid/SQD indexer toolkit — ETL pipelines and GraphQL APIs from onchain archives.",
    function: "data",
    type: "cli",
    repo_url: "https://github.com/subsquid/squid-cli",
    homepage: "https://sqd.dev",
    npm_package: "@subsquid/cli",
    install_command: "npm i -g @subsquid/cli",
    chains: ["ethereum", "sonic", "polkadot"],
    crypto_relevance_score: 82,
  }),
  // --- Wallets ---
  tool({
    slug: "safe-api-kit",
    name: "Safe API Kit",
    description: "Safe Transaction Service TypeScript SDK — multisig proposals and treasury workflows for agents.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/safe-global/safe-core-sdk",
    homepage: "https://docs.safe.global/core-api/transaction-service-overview",
    npm_package: "@safe-global/api-kit",
    install_command: "npm i @safe-global/api-kit",
    chains: ["ethereum", "base", "arbitrum", "polygon"],
    stars: 319,
    license: "MIT",
    requires_secret: true,
    crypto_relevance_score: 86,
  }),
  tool({
    slug: "privy-node-sdk",
    name: "Privy Node.js SDK",
    description: "Server-side Privy SDK — embedded wallets, signing, and agentic wallet backends.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/privy-io/node-sdk",
    homepage: "https://docs.privy.io",
    npm_package: "@privy-io/node",
    install_command: "npm i @privy-io/node",
    chains: ["ethereum", "base", "solana"],
    requires_secret: true,
    crypto_relevance_score: 88,
  }),
  tool({
    slug: "reown-appkit",
    name: "Reown AppKit",
    description: "Reown (WalletConnect) AppKit — multichain wallet connections and embedded wallets for agent UIs.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/reown-com/appkit",
    homepage: "https://docs.reown.com",
    npm_package: "@reown/appkit",
    install_command: "npm i @reown/appkit",
    chains: ["ethereum", "base", "solana"],
    stars: 5400,
    requires_secret: true,
    crypto_relevance_score: 87,
  }),
  tool({
    slug: "rainbowkit",
    name: "RainbowKit",
    description: "React wallet connection toolkit on wagmi/viem — standard dapp wallet UX for onchain agent frontends.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/rainbow-me/rainbowkit",
    homepage: "https://www.rainbowkit.com",
    npm_package: "@rainbow-me/rainbowkit",
    install_command: "npm i @rainbow-me/rainbowkit",
    chains: ["ethereum", "base", "arbitrum"],
    stars: 2800,
    license: "MIT",
    crypto_relevance_score: 80,
  }),
  // --- Portfolio / intelligence APIs ---
  tool({
    slug: "debank-open-api",
    name: "DeBank Open API",
    description: "DeBank Pro API — wallet portfolios, protocol positions, and DeFi user data across EVM chains.",
    function: "data",
    type: "api",
    repo_url: "https://github.com/DeBankDeFi",
    homepage: "https://docs.cloud.debank.com",
    chains: ["ethereum", "base", "arbitrum", "polygon"],
    requires_secret: true,
    install_risk_level: "medium",
    crypto_relevance_score: 82,
  }),
  tool({
    slug: "nansen-api",
    name: "Nansen API",
    description: "Labeled onchain intelligence — Smart Money flows, wallet profiler, and token screener for agents.",
    function: "data",
    type: "api",
    repo_url: "https://github.com/nansen-ai/nansen-cli",
    homepage: "https://docs.nansen.ai",
    chains: ["ethereum", "base", "solana"],
    requires_secret: true,
    install_risk_level: "medium",
    crypto_relevance_score: 89,
  }),
  tool({
    slug: "goldrush-api",
    name: "GoldRush API",
    description: "Covalent GoldRush multichain data APIs — balances, txs, decoded logs across 100+ chains.",
    function: "data",
    type: "sdk",
    repo_url: "https://github.com/covalenthq/goldrush-mcp-server",
    homepage: "https://goldrush.dev/docs",
    npm_package: "@covalenthq/client-sdk",
    install_command: "npm i @covalenthq/client-sdk",
    chains: ["ethereum", "base", "polygon"],
    requires_secret: true,
    crypto_relevance_score: 85,
  }),
  tool({
    slug: "gelato-gasless-sdk",
    name: "Gelato Gasless SDK",
    description: "Gelato relay and automation SDK — gasless transactions and Web3 Functions for agent workflows.",
    function: "dev-tool",
    type: "sdk",
    repo_url: "https://github.com/gelatodigital/gasless",
    homepage: "https://docs.gelato.cloud",
    npm_package: "@gelatocloud/gasless",
    install_command: "npm i @gelatocloud/gasless",
    chains: ["ethereum", "base", "sonic"],
    crypto_relevance_score: 83,
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
  $14, $15, $16,
  $17, $18, $19,
  $20, 'free', $21, $22, 'operator-aggregator-curate-v1',
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
  crypto_relevance_score = EXCLUDED.crypto_relevance_score,
  crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
  relevance_status = EXCLUDED.relevance_status,
  install_risk_level = EXCLUDED.install_risk_level,
  install_risk_reasons = EXCLUDED.install_risk_reasons,
  requires_secret = EXCLUDED.requires_secret,
  license = EXCLUDED.license,
  stars = GREATEST(tools.stars, EXCLUDED.stars),
  source = EXCLUDED.source,
  updated_at = now()
RETURNING slug, (xmax = 0) AS inserted;
`;

if (DRY) {
  console.log(
    JSON.stringify(
      {
        ok: true,
        mode: "dry-run",
        tool_count: TOOLS.length,
        slugs: TOOLS.map((t) => t.slug),
        apply_hint:
          "ENV_FILE=.env SEED_ENV=prod-curate node scripts/seed-crypto-infra-tools.mjs",
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

const DATABASE_URL = env.DATABASE_URL || "";
if (!DATABASE_URL) {
  console.error("DATABASE_URL missing");
  process.exit(2);
}

function pgSslOption(databaseUrl) {
  const mode = (env.PGSSLMODE || "").toLowerCase();
  const wantsSsl =
    mode === "require" ||
    /supabase\.(co|com)/i.test(databaseUrl) ||
    databaseUrl.includes("sslmode=require");
  if (!wantsSsl) return undefined;
  if (env.PG_INSECURE_SSL === "1") return { rejectUnauthorized: false };
  return true;
}

const pg = require(resolve(ROOT, "scripts/ops/node_modules/pg"));
const ssl = pgSslOption(DATABASE_URL);
const client = new pg.Client({
  connectionString: DATABASE_URL,
  ...(ssl !== undefined ? { ssl } : {}),
});

await client.connect();
const results = [];
try {
  for (const t of TOOLS) {
    try {
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
        t.relevance_status,
        t.install_risk_level,
        t.install_risk_reasons,
        t.requires_secret,
        t.license,
        t.stars,
        t.source,
      ]);
      results.push({
        slug: t.slug,
        action: r.rows[0].inserted ? "inserted" : "updated",
      });
    } catch (err) {
      results.push({ slug: t.slug, action: "error", error: err.message });
    }
  }
} finally {
  await client.end();
}
console.log(JSON.stringify({ ok: true, mode: "apply", tools: results }, null, 2));