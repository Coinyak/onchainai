import type { Category, CategoryWithCount } from "@/lib/api";

/** Canonical function labels (matches migrations/001_init.sql seed). */
const FUNCTION_CATEGORY_META: Category[] = [
  {
    id: "bridge",
    label: "Bridge & Cross-chain",
    icon: "git-branch",
    description: "Cross-chain transfers, bridging, wrapping",
    sort_order: 1,
  },
  {
    id: "swap",
    label: "Swap & DEX",
    icon: "arrow-left-right",
    description: "Token swaps, liquidity, routing",
    sort_order: 2,
  },
  {
    id: "wallet",
    label: "Wallet & Custody",
    icon: "credit-card",
    description: "Wallet creation, management, signing, MPC",
    sort_order: 3,
  },
  {
    id: "payments",
    label: "Payments",
    icon: "dollar-sign",
    description: "Payments, x402, transfers, on/offramp",
    sort_order: 4,
  },
  {
    id: "lending",
    label: "Lending & Borrowing",
    icon: "banknote",
    description: "Lending, borrowing, liquidation",
    sort_order: 5,
  },
  {
    id: "staking",
    label: "Staking & Yield",
    icon: "lock",
    description: "Staking, yield, harvesting",
    sort_order: 6,
  },
  {
    id: "trading",
    label: "Trading & Perps",
    icon: "trending-up",
    description: "Trading, perpetuals, options, copy-trade",
    sort_order: 7,
  },
  {
    id: "nft",
    label: "NFT & Marketplace",
    icon: "image",
    description: "NFT viewing, minting, trading",
    sort_order: 8,
  },
  {
    id: "data",
    label: "Data & Analytics",
    icon: "bar-chart",
    description: "Market data, analytics, indexing, oracles",
    sort_order: 9,
  },
  {
    id: "dev-tool",
    label: "Developer Tools",
    icon: "terminal",
    description: "RPC, indexers, contracts, debugging",
    sort_order: 10,
  },
  {
    id: "identity",
    label: "Identity & KYA",
    icon: "fingerprint",
    description: "Onchain identity, attestation, agent auth",
    sort_order: 11,
  },
  {
    id: "governance",
    label: "Governance & DAO",
    icon: "vote",
    description: "Voting, proposals, treasury",
    sort_order: 12,
  },
  {
    id: "social",
    label: "Social & Content",
    icon: "message-circle",
    description: "Decentralized social, content, creators",
    sort_order: 13,
  },
  {
    id: "ai-agent",
    label: "AI Agent",
    icon: "bot",
    description: "Autonomous agents, agent economy, DeFAI",
    sort_order: 14,
  },
];

export const FUNCTION_CATEGORY_FALLBACK: CategoryWithCount[] = FUNCTION_CATEGORY_META.map(
  (category) => ({
    category,
    count: 0,
  }),
);