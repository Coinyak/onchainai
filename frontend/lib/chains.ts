export interface ChainMeta {
  id: string;
  label: string;
  logo: string;
  aliases: string[];
  pinned: boolean;
}

export const CHAIN_CATALOG: ChainMeta[] = [
  { id: "bitcoin", label: "Bitcoin", logo: "/chains/bitcoin.svg", aliases: ["btc", "btc-mainnet", "xbt"], pinned: true },
  { id: "bob", label: "BOB", logo: "/chains/bob.svg", aliases: ["build-on-bitcoin", "gobob"], pinned: true },
  { id: "ethereum", label: "Ethereum", logo: "/chains/ethereum.svg", aliases: ["eth", "eth-mainnet", "ethereum-mainnet"], pinned: false },
  { id: "solana", label: "Solana", logo: "/chains/solana.svg", aliases: ["sol", "solana-mainnet"], pinned: false },
  { id: "base", label: "Base", logo: "/chains/base.svg", aliases: ["base-mainnet", "coinbase-base"], pinned: false },
  { id: "arbitrum", label: "Arbitrum", logo: "/chains/arbitrum.svg", aliases: ["arb", "arbitrum-one", "arb-one"], pinned: false },
  { id: "optimism", label: "Optimism", logo: "/chains/optimism.svg", aliases: ["op", "optimism-mainnet", "op-mainnet"], pinned: false },
  { id: "polygon", label: "Polygon", logo: "/chains/polygon.svg", aliases: ["matic", "polygon-pos", "polygon-mainnet", "matic-mainnet"], pinned: false },
  { id: "bsc", label: "BNB Chain", logo: "/chains/bsc.svg", aliases: ["bnb", "binance", "binance-smart-chain", "bnb-chain", "bnb-smart-chain", "binance-chain"], pinned: false },
  { id: "avalanche", label: "Avalanche", logo: "/chains/avalanche.svg", aliases: ["avax", "avalanche-c", "avax-c", "c-chain"], pinned: false },
  { id: "sui", label: "Sui", logo: "/chains/sui.svg", aliases: ["sui-mainnet"], pinned: false },
  { id: "zksync", label: "zkSync", logo: "/chains/zksync.svg", aliases: ["zk-sync", "zksync-era", "zksync-mainnet", "zk-sync-era"], pinned: false },
  { id: "sonic", label: "Sonic", logo: "/chains/sonic.svg", aliases: ["sonic-mainnet"], pinned: false },
  { id: "unichain", label: "Unichain", logo: "/chains/unichain.svg", aliases: ["uni-chain"], pinned: false },
  { id: "bera", label: "Berachain", logo: "/chains/bera.svg", aliases: ["berachain", "berachain-mainnet"], pinned: false },
  { id: "sei", label: "Sei", logo: "/chains/sei.svg", aliases: ["sei-network", "sei-mainnet"], pinned: false },
  { id: "soneium", label: "Soneium", logo: "/chains/soneium.svg", aliases: ["soneium-mainnet"], pinned: false },
  { id: "tron", label: "Tron", logo: "/chains/tron.svg", aliases: ["trx", "tron-mainnet"], pinned: false },
  { id: "hyperliquid", label: "Hyperliquid", logo: "/chains/hyperliquid.svg", aliases: ["hype", "hyperliquid-xyz", "hl"], pinned: false },
  { id: "plasma", label: "Plasma", logo: "/chains/plasma.svg", aliases: ["plasma-mainnet"], pinned: false },
  { id: "linea", label: "Linea", logo: "/chains/linea.svg", aliases: ["linea-mainnet"], pinned: false },
  { id: "starknet", label: "Starknet", logo: "/chains/starknet.svg", aliases: ["stark", "starknet-mainnet"], pinned: false },
  { id: "aptos", label: "Aptos", logo: "/chains/aptos.svg", aliases: ["apt", "aptos-mainnet"], pinned: false },
  { id: "near", label: "NEAR", logo: "/chains/near.svg", aliases: ["near-protocol", "near-mainnet"], pinned: false },
  { id: "cosmos", label: "Cosmos", logo: "/chains/cosmos.svg", aliases: ["cosmos-hub", "atom", "cosmos-mainnet"], pinned: false },
  { id: "celo", label: "Celo", logo: "/chains/celo.svg", aliases: ["celo-mainnet"], pinned: false },
  { id: "fantom", label: "Fantom", logo: "/chains/fantom.svg", aliases: ["ftm", "fantom-mainnet"], pinned: false },
  { id: "blast", label: "Blast", logo: "/chains/blast.svg", aliases: ["blast-mainnet"], pinned: false },
  { id: "scroll", label: "Scroll", logo: "/chains/scroll.svg", aliases: ["scroll-mainnet"], pinned: false },
  { id: "gnosis", label: "Gnosis", logo: "/chains/gnosis.svg", aliases: ["gno", "xdai"], pinned: false },
  { id: "cardano", label: "Cardano", logo: "/chains/cardano.svg", aliases: ["ada", "cardano-mainnet"], pinned: false },
  { id: "moonbeam", label: "Moonbeam", logo: "/chains/moonbeam.svg", aliases: ["glmr", "moonbeam-mainnet"], pinned: false },
  { id: "litecoin", label: "Litecoin", logo: "/chains/litecoin.svg", aliases: ["ltc", "litecoin-mainnet"], pinned: false },
  { id: "bitcoin-sv", label: "Bitcoin SV", logo: "/chains/bitcoin-sv.svg", aliases: ["bsv", "bitcoinsv", "bitcoin-sv-mainnet"], pinned: false },
  { id: "lightning", label: "Lightning", logo: "/chains/lightning.svg", aliases: ["bitcoin-lightning", "ln", "lightning-network"], pinned: false },
  { id: "x-layer", label: "X Layer", logo: "/chains/x-layer.svg", aliases: ["xlayer", "okx-x-layer", "x-layer-mainnet"], pinned: false },
  { id: "dogecoin", label: "Dogecoin", logo: "/chains/dogecoin.svg", aliases: ["doge", "dogecoin-mainnet"], pinned: false },
  { id: "aurora", label: "Aurora", logo: "/chains/aurora.svg", aliases: ["aurora-mainnet"], pinned: false },
  { id: "okx", label: "OKX Chain", logo: "/chains/okx.svg", aliases: ["okc", "okex-chain"], pinned: false },
  { id: "monad", label: "Monad", logo: "/chains/monad.svg", aliases: ["mon", "monad-mainnet"], pinned: false },
  { id: "mantle", label: "Mantle", logo: "/chains/mantle.svg", aliases: ["mnt", "mantle-mainnet"], pinned: false },
  { id: "cronos", label: "Cronos", logo: "/chains/cronos.svg", aliases: ["cro", "crypto-com-chain", "cronos-mainnet"], pinned: false },
  { id: "movement", label: "Movement", logo: "/chains/movement.svg", aliases: ["move", "movement-mainnet", "movement-labs"], pinned: false },
  { id: "ink", label: "Ink", logo: "/chains/ink.svg", aliases: ["ink-mainnet", "inkchain"], pinned: false },
  { id: "flare", label: "Flare", logo: "/chains/flare.svg", aliases: ["flr", "flare-networks", "flare-mainnet"], pinned: false },
  { id: "rootstock", label: "Rootstock", logo: "/chains/rootstock.svg", aliases: ["rsk", "rbtc"], pinned: false },
  { id: "megaeth", label: "MegaETH", logo: "/chains/megaeth.svg", aliases: ["mega-eth"], pinned: false },
  { id: "stacks", label: "Stacks", logo: "/chains/stacks.svg", aliases: ["stx", "blockstack"], pinned: false },
  { id: "polkadot", label: "Polkadot", logo: "/chains/polkadot.svg", aliases: ["dot", "polkadot-relay"], pinned: false },
  { id: "kava", label: "Kava", logo: "/chains/kava.svg", aliases: ["kava-mainnet"], pinned: false },
  { id: "ton", label: "TON", logo: "/chains/ton.svg", aliases: ["the-open-network", "ton-mainnet"], pinned: false },
  { id: "taiko", label: "Taiko", logo: "/chains/taiko.svg", aliases: ["taiko-mainnet"], pinned: false },
  { id: "immutable", label: "Immutable", logo: "/chains/immutable.svg", aliases: ["imx", "immutablex", "immutable-x", "immutable-zkevm"], pinned: false },
  { id: "zora", label: "Zora", logo: "/chains/zora.svg", aliases: ["zora-mainnet"], pinned: false },
  { id: "stellar", label: "Stellar", logo: "/chains/stellar.svg", aliases: ["xlm", "stellar-mainnet"], pinned: false },
  { id: "algorand", label: "Algorand", logo: "/chains/algorand.svg", aliases: ["algo", "algorand-mainnet"], pinned: false },
  { id: "filecoin", label: "Filecoin", logo: "/chains/filecoin.svg", aliases: ["fil", "filecoin-mainnet"], pinned: false },
  { id: "ronin", label: "Ronin", logo: "/chains/ronin.svg", aliases: ["ron", "ronin-mainnet"], pinned: false },
  { id: "worldchain", label: "World Chain", logo: "/chains/worldchain.svg", aliases: ["world-chain", "worldcoin", "wld"], pinned: false },
  { id: "hedera", label: "Hedera", logo: "/chains/hedera.svg", aliases: ["hbar", "hedera-hashgraph"], pinned: false },
  { id: "xrpl", label: "XRPL", logo: "/chains/xrpl.svg", aliases: ["xrp", "ripple", "xrpl-mainnet"], pinned: false },
  { id: "thorchain", label: "THORChain", logo: "/chains/thorchain.svg", aliases: ["rune", "thorchain-mainnet"], pinned: false },
  { id: "katana", label: "Katana", logo: "/chains/katana.svg", aliases: ["katana-mainnet"], pinned: false },
  { id: "dydx", label: "dYdX", logo: "/chains/dydx.svg", aliases: ["dydx-chain", "dydx-mainnet"], pinned: false },
  { id: "fraxtal", label: "Fraxtal", logo: "/chains/fraxtal.svg", aliases: ["frax", "fraxtal-mainnet"], pinned: false },
  { id: "tezos", label: "Tezos", logo: "/chains/tezos.svg", aliases: ["xtz", "tezos-mainnet"], pinned: false },
  { id: "mezo", label: "Mezo", logo: "/chains/mezo.svg", aliases: ["mezo-mainnet"], pinned: false },
  { id: "bittensor", label: "Bittensor", logo: "/chains/bittensor.svg", aliases: ["tao", "bittensor-mainnet"], pinned: false },
  { id: "pulsechain", label: "PulseChain", logo: "/chains/pulsechain.svg", aliases: ["pls", "pulsechain-mainnet"], pinned: false },
  { id: "provenance", label: "Provenance", logo: "/chains/provenance.svg", aliases: ["hash", "hash-2", "provenance-mainnet"], pinned: false },
  { id: "fluent", label: "Fluent", logo: "/chains/fluent.svg", aliases: ["fluent-network", "fluent-mainnet"], pinned: false },
  { id: "hydration", label: "Hydration", logo: "/chains/hydration.svg", aliases: ["hydradx", "hydration-mainnet"], pinned: false },
  { id: "mixin", label: "Mixin", logo: "/chains/mixin.svg", aliases: ["xin", "mixin-mainnet"], pinned: false },
  { id: "vaulta", label: "Vaulta", logo: "/chains/vaulta.svg", aliases: ["eos", "eosio", "vaulta-mainnet"], pinned: false },
  { id: "ethereal", label: "Ethereal", logo: "/chains/ethereal.svg", aliases: ["ethereal-mainnet"], pinned: false },
  { id: "stable", label: "Stable", logo: "/chains/stable.svg", aliases: ["stable-2", "stable-mainnet"], pinned: false },
  { id: "xpr", label: "XPR Network", logo: "/chains/xpr.svg", aliases: ["proton", "xpr-network", "xpr-mainnet"], pinned: false },
];

export const STRIP_PRIMARY_VISIBLE = 20;

/** Bump when regenerating `public/chains` tiles (sync with scripts/chain-logo-manifest.json harness_round). */
export const CHAIN_LOGO_ASSET_VERSION = "12";

/** Chain IDs with a committed `/chains/<id>.svg` tile (no text-fallback tiles in strip). */
export const CHAIN_LOGO_IDS = new Set<string>(
  CHAIN_CATALOG.map((entry) => entry.id),
);

export function hasChainLogo(id: string): boolean {
  return CHAIN_LOGO_IDS.has(id.trim().toLowerCase());
}

const CHAIN_NOISE = new Set([
  "all", "any", "none", "unknown", "multi-chain", "multichain", "multi",
  "cross-chain", "crosschain", "omnichain", "omni-chain", "ecosystem",
]);

const CHAIN_SUFFIXES = ["-mainnet", "-testnet", "-network", "-one", "-pos", "-era"];

export function normalizeChainKey(raw: string): string {
  let key = raw.trim().toLowerCase().replace(/[_ ]/g, "-");
  while (key.includes("--")) key = key.replace("--", "-");
  key = key.replace(/^-+|-+$/g, "");

  let stripped = true;
  while (stripped) {
    stripped = false;
    for (const suffix of CHAIN_SUFFIXES) {
      if (key.endsWith(suffix) && key.length > suffix.length) {
        key = key.slice(0, -suffix.length);
        stripped = true;
      }
    }
  }
  return key;
}

export function isChainNoise(raw: string): boolean {
  const key = normalizeChainKey(raw);
  if (!key) return true;
  if (CHAIN_NOISE.has(key)) return true;
  return key.includes("+") || key.includes("networks");
}

export function resolveChain(dbValue: string): ChainMeta | undefined {
  const normalized = normalizeChainKey(dbValue);
  if (!normalized || isChainNoise(normalized)) return undefined;
  return CHAIN_CATALOG.find(
    (entry) =>
      entry.id === normalized ||
      entry.aliases.some((a) => normalizeChainKey(a) === normalized),
  );
}

export function chainLogoPath(id: string): string {
  return `/chains/${id.trim().toLowerCase()}.svg?v=${CHAIN_LOGO_ASSET_VERSION}`;
}

export function chainFallbackLabel(raw: string): string {
  const resolved = resolveChain(raw);
  if (resolved) return resolved.label;
  return raw
    .split(/[-_]/)
    .filter(Boolean)
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join(" ");
}

export function chainTagsForTool(chains: string[]): ChainMeta[] {
  const seen = new Set<string>();
  const result: ChainMeta[] = [];
  for (const raw of chains) {
    const meta = resolveChain(raw) ?? syntheticChainMeta(raw);
    if (!meta || seen.has(meta.id)) continue;
    seen.add(meta.id);
    result.push(meta);
  }
  return result;
}

function syntheticChainMeta(raw: string): ChainMeta | undefined {
  if (isChainNoise(raw)) return undefined;
  const id = normalizeChainKey(raw);
  if (!id) return undefined;
  const catalogMatch = CHAIN_CATALOG.find((c) => c.id === id);
  if (catalogMatch) return catalogMatch;
  return {
    id,
    label: chainFallbackLabel(raw),
    logo: hasChainLogo(id) ? chainLogoPath(id) : "",
    aliases: [raw],
    pinned: false,
  };
}

export function stripChains(chainCounts: [string, number][]): ChainMeta[] {
  const byId = new Map<string, { meta: ChainMeta; count: number }>();

  for (const [raw, count] of chainCounts) {
    const meta = resolveChain(raw) ?? syntheticChainMeta(raw);
    if (!meta) continue;
    const prev = byId.get(meta.id);
    if (prev) {
      prev.count += count;
    } else {
      byId.set(meta.id, { meta, count });
    }
  }

  const pinned = CHAIN_CATALOG.filter((c) => c.pinned && byId.has(c.id));
  const pinnedIds = new Set(pinned.map((c) => c.id));
  const rest = [...byId.values()]
    .filter((entry) => !pinnedIds.has(entry.meta.id))
    .sort((a, b) => b.count - a.count)
    .map((entry) => entry.meta);

  return [...pinned, ...rest];
}

export function chainFilterActive(entry: ChainMeta, active: string[]): boolean {
  return active.some((v) => {
    const norm = normalizeChainKey(v);
    return norm === entry.id || entry.aliases.some((a) => normalizeChainKey(a) === norm);
  });
}