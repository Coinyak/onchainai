#!/usr/bin/env node
/**
 * Discover chain + tool candidates from DeFiLlama (primary) and CoinGecko (bulk).
 * Default: dry-run JSON to stdout — no DB writes.
 *
 *   node scripts/seed-from-aggregators.mjs
 *   node scripts/seed-from-aggregators.mjs --chains
 *   node scripts/seed-from-aggregators.mjs --tools
 *   node scripts/seed-from-aggregators.mjs --tools --limit 30
 *
 * Apply curated infra tools (separate script, DB gate):
 *   ENV_FILE=.env SEED_ENV=prod-curate node scripts/seed-crypto-infra-tools.mjs
 */

const LLAMA_CHAINS = "https://api.llama.fi/v2/chains";
const LLAMA_PROTOCOLS = "https://api.llama.fi/protocols";
const CG_PLATFORMS = "https://api.coingecko.com/api/v3/asset_platforms";

/** Synced with src/chains.rs CHAIN_CATALOG ids (harness-round-13, 74 chains). */
const CATALOG_IDS = new Set([
  "bitcoin", "bob", "ethereum", "solana", "base", "arbitrum", "optimism",
  "polygon", "bsc", "avalanche", "sui", "zksync", "sonic", "unichain",
  "bera", "sei", "soneium", "tron", "hyperliquid", "plasma", "linea",
  "starknet", "aptos", "near", "cosmos", "celo", "fantom", "blast",
  "scroll", "gnosis", "cardano", "moonbeam", "litecoin", "dogecoin",
  "aurora", "okx", "monad", "mantle", "cronos", "movement", "ink",
  "flare", "rootstock", "megaeth", "stacks", "polkadot", "kava", "ton",
  "taiko", "immutable", "zora", "stellar", "algorand", "filecoin", "ronin",
  "worldchain", "hedera", "xrpl", "thorchain", "katana", "dydx", "fraxtal",
  "tezos", "mezo", "bittensor", "pulsechain", "provenance", "fluent",
  "hydration", "mixin", "vaulta", "ethereal", "stable", "xpr", "robinhood",
]);

const CHAIN_SLUG_MAP = {
  "op mainnet": "optimism",
  "bnb chain": "bsc",
  "binance": "bsc",
  "immutable zkevm": "immutable",
  "x layer": "okx",
};

const TOOL_KEYWORDS = [
  "oracle", "index", "graph", "rpc", "bridge", "layerzero", "wormhole",
  "hyperlane", "safe", "wallet", "api", "infrastructure", "data",
  "automation", "gelato", "pyth", "chainlink", "subsquid", "goldsky",
];

function normalizeId(name) {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function catalogHas(name, geckoId) {
  const n = normalizeId(name);
  if (CATALOG_IDS.has(n)) return true;
  if (CHAIN_SLUG_MAP[n] && CATALOG_IDS.has(CHAIN_SLUG_MAP[n])) return true;
  if (geckoId && CATALOG_IDS.has(geckoId.replace(/-chain$/, ""))) return true;
  return [...CATALOG_IDS].some(
    (id) => n.includes(id) || id.includes(n),
  );
}

async function fetchJson(url, opts = {}) {
  const res = await fetch(url, {
    headers: { "User-Agent": "OnchainAI-aggregator-discovery/1.0" },
    ...opts,
  });
  if (!res.ok) throw new Error(`${url} → ${res.status}`);
  return res.json();
}

function chainCandidate(row) {
  const id = normalizeId(row.name);
  return {
    kind: "chain",
    id,
    label: row.name,
    chainId: row.chainId ?? null,
    gecko_id: row.gecko_id ?? null,
    tokenSymbol: row.tokenSymbol ?? null,
    tvl_usd: row.tvl ?? 0,
    logo_hint: `https://icons.llama.fi/${id}.png`,
    sources: ["defillama"],
    action: "review_before_catalog_add",
  };
}

function toolCandidate(p) {
  const slug = normalizeId(p.slug || p.name);
  return {
    kind: "tool",
    slug,
    name: p.name,
    homepage: p.url || null,
    defillama_slug: p.slug || null,
    category: p.category || null,
    chains: (p.chains || []).slice(0, 8),
    logo_hint: p.slug ? `https://icons.llama.fi/${p.slug}.png` : null,
    tvl_usd: p.tvl ?? 0,
    sources: ["defillama"],
    action: "review_before_seed",
  };
}

function toolRelevant(p) {
  const blob = `${p.name} ${p.category} ${p.slug}`.toLowerCase();
  if (TOOL_KEYWORDS.some((k) => blob.includes(k))) return true;
  const skip = ["lending", "dex", "yield", "farm", "staking", "cdp", "derivatives"];
  return !skip.some((k) => (p.category || "").toLowerCase().includes(k));
}

async function discoverChains(limit = 25) {
  const rows = await fetchJson(LLAMA_CHAINS);
  return rows
    .filter((r) => (r.tvl || 0) > 5_000_000)
    .filter((r) => !catalogHas(r.name, r.gecko_id))
    .sort((a, b) => (b.tvl || 0) - (a.tvl || 0))
    .slice(0, limit)
    .map(chainCandidate);
}

async function discoverTools(limit = 40) {
  const rows = await fetchJson(LLAMA_PROTOCOLS);
  return rows
    .filter(toolRelevant)
    .filter((p) => (p.tvl || 0) > 1_000_000 || toolRelevant(p))
    .sort((a, b) => (b.tvl || 0) - (a.tvl || 0))
    .slice(0, limit * 2)
    .filter((p, i, arr) => arr.findIndex((x) => normalizeId(x.slug) === normalizeId(p.slug)) === i)
    .slice(0, limit)
    .map(toolCandidate);
}

async function discoverGeckoPlatforms() {
  try {
    const plats = await fetchJson(CG_PLATFORMS);
    return plats
      .filter((p) => p.chain_identifier && !CATALOG_IDS.has(p.id))
      .slice(0, 20)
      .map((p) => ({
        kind: "chain_hint",
        coingecko_platform_id: p.id,
        chain_identifier: p.chain_identifier,
        native_coin_id: p.native_coin_id,
        sources: ["coingecko"],
      }));
  } catch (e) {
    return { error: String(e), hint: "CoinGecko rate-limited; use DeFiLlama primary" };
  }
}

const args = process.argv.slice(2);
const chainsOnly = args.includes("--chains") || args.length === 0;
const toolsOnly = args.includes("--tools");
const limitIdx = args.indexOf("--limit");
const limit = limitIdx >= 0 ? Number(args[limitIdx + 1]) || 25 : 25;

const out = {
  mode: "dry-run",
  catalog_size: CATALOG_IDS.size,
  discovered_at: new Date().toISOString(),
  chains: [],
  tools: [],
  coingecko_hints: null,
};

if (!toolsOnly) {
  out.chains = await discoverChains(limit);
}
if (!chainsOnly || toolsOnly) {
  out.tools = await discoverTools(limit);
}
if (args.includes("--gecko")) {
  out.coingecko_hints = await discoverGeckoPlatforms();
}

console.log(JSON.stringify(out, null, 2));