#!/usr/bin/env node
// Circle for Agents — operator-curated Circle ecosystem tools (agents.circle.com).
// Excludes circlefin/skills (slug collision pre-check). circle-x402-batching has no
// public repo — stays community until verify; promotion only via verify-tool-official.
//
//   node scripts/seed-circle-agent-tools.mjs
//   ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-circle-agent-tools.mjs

import { tool, runSeed } from "./seed-tool-lib.mjs";

const TOOLS = [
  tool({
    slug: "circle-agent-stack",
    name: "Circle Agent Stack",
    description:
      "Circle starter kits for AI agents — payments, wallets, and x402 integration patterns from agents.circle.com.",
    function: "payments",
    type: "sdk",
    repo_url: "https://github.com/circlefin/agent-stack-starter-kits",
    homepage: "https://agents.circle.com",
    crypto_relevance_score: 90,
    crypto_relevance_reasons: [
      "official Circle agent integration starter kits",
      "operator-curated payments surface for ai-agent workflows",
    ],
  }),
  tool({
    slug: "circle-x402-batching",
    name: "Circle x402 Batching",
    description:
      "Gasless, batched settlement for x402 payments via Circle Gateway — npm-only; no public GitHub repo.",
    function: "payments",
    type: "sdk",
    homepage: "https://developers.circle.com",
    npm_package: "@circle-fin/x402-batching",
    install_command: "npm i @circle-fin/x402-batching",
    license: "Apache-2.0",
    crypto_relevance_score: 88,
    crypto_relevance_reasons: [
      "official Circle x402 batching SDK (npm @circle-fin scope)",
      "community until public repo is published",
    ],
  }),
  tool({
    slug: "circle-gateway",
    name: "Circle Gateway",
    description:
      "Circle Gateway EVM contracts — unified USDC balance and settlement for cross-chain agent payments.",
    function: "payments",
    type: "api",
    repo_url: "https://github.com/circlefin/evm-gateway-contracts",
    homepage: "https://developers.circle.com",
    chains: ["ethereum", "base", "arbitrum", "avalanche", "polygon"],
    crypto_relevance_score: 89,
  }),
  tool({
    slug: "circle-cctp-v2",
    name: "Circle CCTP v2",
    description:
      "Cross-Chain Transfer Protocol v2 EVM contracts — native USDC bridging across supported chains.",
    function: "bridge",
    type: "api",
    repo_url: "https://github.com/circlefin/evm-cctp-contracts",
    homepage: "https://developers.circle.com",
    chains: ["ethereum", "base", "arbitrum", "avalanche", "polygon", "optimism"],
    crypto_relevance_score: 91,
    crypto_relevance_reasons: [
      "official Circle CCTP v2 cross-chain USDC bridge contracts",
      "operator-curated bridge surface for onchain agents",
    ],
  }),
  tool({
    slug: "circle-cctp-provider-sdk",
    name: "Circle CCTP Provider SDK",
    description:
      "Circle's official Cross-Chain Transfer Protocol v2 provider SDK for native USDC bridging in Node.js apps.",
    function: "bridge",
    type: "sdk",
    homepage: "https://developers.circle.com",
    npm_package: "@circle-fin/provider-cctp-v2",
    install_command: "npm i @circle-fin/provider-cctp-v2",
    chains: ["ethereum", "base", "arbitrum", "avalanche", "polygon", "optimism"],
    crypto_relevance_score: 90,
  }),
  tool({
    slug: "circle-dev-controlled-wallets",
    name: "Circle Developer Controlled Wallets",
    description:
      "Node.js SDK for Circle Developer Controlled Wallets — server-side wallet custody for agent backends.",
    function: "wallet",
    type: "sdk",
    homepage: "https://developers.circle.com/api-reference/wallets/common/ping",
    npm_package: "@circle-fin/developer-controlled-wallets",
    install_command: "npm i @circle-fin/developer-controlled-wallets",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: [
      "requires Circle API key for wallet operations",
      "documented package manager install",
    ],
    crypto_relevance_score: 87,
  }),
  tool({
    slug: "circle-user-controlled-wallets",
    name: "Circle User Controlled Wallets",
    description:
      "Node.js SDK for Circle User Controlled Wallets — end-user wallet flows for agent-facing applications.",
    function: "wallet",
    type: "sdk",
    homepage: "https://developers.circle.com/api-reference/wallets/common/ping",
    npm_package: "@circle-fin/user-controlled-wallets",
    install_command: "npm i @circle-fin/user-controlled-wallets",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: [
      "requires Circle API key for wallet operations",
      "documented package manager install",
    ],
    crypto_relevance_score: 86,
  }),
  tool({
    slug: "circle-modular-wallets",
    name: "Circle Modular Wallets",
    description:
      "Circle Modular Wallets web SDK — smart-account wallets with passkey auth and gas abstraction for agents.",
    function: "wallet",
    type: "sdk",
    repo_url: "https://github.com/circlefin/modularwallets-web-sdk",
    homepage: "https://developers.circle.com/wallets/modular",
    npm_package: "@circle-fin/modular-wallets-core",
    install_command: "npm i @circle-fin/modular-wallets-core",
    crypto_relevance_score: 88,
  }),
  tool({
    slug: "circle-paymaster",
    name: "Circle Paymaster",
    description:
      "Circle Paymaster service — sponsor gas fees in USDC for smart-account transactions in agent workflows.",
    function: "payments",
    type: "api",
    homepage: "https://developers.circle.com/paymaster",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: ["requires Circle API key for paymaster operations"],
    crypto_relevance_score: 87,
  }),
  tool({
    slug: "circle-api-node-sdk",
    name: "Circle API Node SDK",
    description:
      "Official Circle Node.js SDK — programmatic access to Circle payments, wallets, and treasury APIs.",
    function: "payments",
    type: "sdk",
    repo_url: "https://github.com/circlefin/circle-nodejs-sdk",
    homepage: "https://developers.circle.com",
    npm_package: "@circle-fin/circle-sdk",
    install_command: "npm i @circle-fin/circle-sdk",
    requires_secret: true,
    install_risk_level: "medium",
    install_risk_reasons: [
      "requires Circle API key for signed endpoints",
      "documented package manager install",
    ],
    crypto_relevance_score: 88,
  }),
  tool({
    slug: "usdc-stablecoin-contracts",
    name: "USDC Stablecoin Contracts",
    description:
      "Circle USDC EVM smart contracts — canonical stablecoin token implementations across supported chains.",
    function: "dev-tool",
    type: "sdk",
    asset_class: "stablecoins",
    repo_url: "https://github.com/circlefin/stablecoin-evm",
    homepage: "https://developers.circle.com",
    chains: ["ethereum", "base", "arbitrum", "avalanche", "polygon", "optimism"],
    crypto_relevance_score: 90,
    crypto_relevance_reasons: [
      "official Circle USDC EVM contract source",
      "operator-curated stablecoin reference for onchain agents",
    ],
  }),
];

await runSeed(TOOLS, "seed-circle-agent-tools.mjs");