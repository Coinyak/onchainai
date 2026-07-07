//! Chain catalog — allowlist for logo strip and tool-card tags.
//! Official logo markers validated via scripts/chain-logo-manifest.json (harness-round-11).

/// Metadata for a supported blockchain in the UI.
#[derive(Debug, Clone, Copy)]
pub struct ChainMeta {
    /// Canonical id; matches a DB `chains[]` value when present.
    pub id: &'static str,
    /// Accessible name for aria-label, title, and alt text.
    pub label: &'static str,
    /// Public logo path under `/chains/`.
    pub logo: &'static str,
    /// Other DB values that resolve to this entry.
    pub aliases: &'static [&'static str],
    /// Always shown in the strip, even when tool count is zero.
    pub pinned: bool,
}

/// Ordered chain allowlist. Bitcoin first (pinned), then BOB (pinned), then the rest.
pub const CHAIN_CATALOG: &[ChainMeta] = &[
    ChainMeta {
        id: "bitcoin",
        label: "Bitcoin",
        logo: "/chains/bitcoin.svg",
        aliases: &["btc", "btc-mainnet", "xbt"],
        pinned: true,
    },
    ChainMeta {
        id: "bob",
        label: "BOB",
        logo: "/chains/bob.svg",
        aliases: &["build-on-bitcoin", "gobob"],
        pinned: true,
    },
    ChainMeta {
        id: "ethereum",
        label: "Ethereum",
        logo: "/chains/ethereum.svg",
        aliases: &["eth", "eth-mainnet", "ethereum-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "solana",
        label: "Solana",
        logo: "/chains/solana.svg",
        aliases: &["sol", "solana-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "base",
        label: "Base",
        logo: "/chains/base.svg",
        aliases: &["base-mainnet", "coinbase-base"],
        pinned: false,
    },
    ChainMeta {
        id: "arbitrum",
        label: "Arbitrum",
        logo: "/chains/arbitrum.svg",
        aliases: &["arb", "arbitrum-one", "arb-one"],
        pinned: false,
    },
    ChainMeta {
        id: "optimism",
        label: "Optimism",
        logo: "/chains/optimism.svg",
        aliases: &["op", "optimism-mainnet", "op-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "polygon",
        label: "Polygon",
        logo: "/chains/polygon.svg",
        aliases: &["matic", "polygon-pos", "polygon-mainnet", "matic-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "bsc",
        label: "BNB Chain",
        logo: "/chains/bsc.svg",
        aliases: &[
            "bnb",
            "binance",
            "binance-smart-chain",
            "bnb-chain",
            "bnb-smart-chain",
            "binance-chain",
        ],
        pinned: false,
    },
    ChainMeta {
        id: "avalanche",
        label: "Avalanche",
        logo: "/chains/avalanche.svg",
        aliases: &["avax", "avalanche-c", "avax-c", "c-chain"],
        pinned: false,
    },
    ChainMeta {
        id: "sui",
        label: "Sui",
        logo: "/chains/sui.svg",
        aliases: &["sui-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "zksync",
        label: "zkSync",
        logo: "/chains/zksync.svg",
        aliases: &["zk-sync", "zksync-era", "zksync-mainnet", "zk-sync-era"],
        pinned: false,
    },
    ChainMeta {
        id: "sonic",
        label: "Sonic",
        logo: "/chains/sonic.svg",
        aliases: &["sonic-mainnet", "fantom", "ftm", "fantom-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "unichain",
        label: "Unichain",
        logo: "/chains/unichain.svg",
        aliases: &["uni-chain"],
        pinned: false,
    },
    ChainMeta {
        id: "bera",
        label: "Berachain",
        logo: "/chains/bera.svg",
        aliases: &["berachain", "berachain-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "sei",
        label: "Sei",
        logo: "/chains/sei.svg",
        aliases: &["sei-network", "sei-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "soneium",
        label: "Soneium",
        logo: "/chains/soneium.svg",
        aliases: &["soneium-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "tron",
        label: "Tron",
        logo: "/chains/tron.svg",
        aliases: &["trx", "tron-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "hyperliquid",
        label: "Hyperliquid",
        logo: "/chains/hyperliquid.svg",
        aliases: &["hype", "hyperliquid-xyz", "hl"],
        pinned: false,
    },
    ChainMeta {
        id: "plasma",
        label: "Plasma",
        logo: "/chains/plasma.svg",
        aliases: &["plasma-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "linea",
        label: "Linea",
        logo: "/chains/linea.svg",
        aliases: &["linea-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "starknet",
        label: "Starknet",
        logo: "/chains/starknet.svg",
        aliases: &["stark", "starknet-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "aptos",
        label: "Aptos",
        logo: "/chains/aptos.svg",
        aliases: &["apt", "aptos-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "near",
        label: "NEAR",
        logo: "/chains/near.svg",
        aliases: &["near-protocol", "near-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "cosmos",
        label: "Cosmos",
        logo: "/chains/cosmos.svg",
        aliases: &["cosmos-hub", "atom", "cosmos-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "celo",
        label: "Celo",
        logo: "/chains/celo.svg",
        aliases: &["celo-mainnet"],
        pinned: false,
    },
    // Fantom rebranded to Sonic (2025); `fantom`/`ftm` are aliases of `sonic`.
    ChainMeta {
        id: "blast",
        label: "Blast",
        logo: "/chains/blast.svg",
        aliases: &["blast-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "scroll",
        label: "Scroll",
        logo: "/chains/scroll.svg",
        aliases: &["scroll-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "gnosis",
        label: "Gnosis",
        logo: "/chains/gnosis.svg",
        aliases: &["gno", "xdai"],
        pinned: false,
    },
    ChainMeta {
        id: "cardano",
        label: "Cardano",
        logo: "/chains/cardano.svg",
        aliases: &["ada", "cardano-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "moonbeam",
        label: "Moonbeam",
        logo: "/chains/moonbeam.svg",
        aliases: &["glmr", "moonbeam-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "litecoin",
        label: "Litecoin",
        logo: "/chains/litecoin.svg",
        aliases: &["ltc", "litecoin-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "dogecoin",
        label: "Dogecoin",
        logo: "/chains/dogecoin.svg",
        aliases: &["doge", "dogecoin-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "aurora",
        label: "Aurora",
        logo: "/chains/aurora.svg",
        aliases: &["aurora-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "okx",
        label: "OKX Chain",
        logo: "/chains/okx.svg",
        aliases: &["okc", "okex-chain"],
        pinned: false,
    },
    ChainMeta {
        id: "monad",
        label: "Monad",
        logo: "/chains/monad.svg",
        aliases: &["mon", "monad-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "mantle",
        label: "Mantle",
        logo: "/chains/mantle.svg",
        aliases: &["mnt", "mantle-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "cronos",
        label: "Cronos",
        logo: "/chains/cronos.svg",
        aliases: &["cro", "crypto-com-chain", "cronos-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "movement",
        label: "Movement",
        logo: "/chains/movement.svg",
        aliases: &["move", "movement-mainnet", "movement-labs"],
        pinned: false,
    },
    ChainMeta {
        id: "ink",
        label: "Ink",
        logo: "/chains/ink.svg",
        aliases: &["ink-mainnet", "inkchain"],
        pinned: false,
    },
    ChainMeta {
        id: "flare",
        label: "Flare",
        logo: "/chains/flare.svg",
        aliases: &["flr", "flare-networks", "flare-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "rootstock",
        label: "Rootstock",
        logo: "/chains/rootstock.svg",
        aliases: &["rsk", "rbtc"],
        pinned: false,
    },
    ChainMeta {
        id: "megaeth",
        label: "MegaETH",
        logo: "/chains/megaeth.svg",
        aliases: &["mega-eth"],
        pinned: false,
    },
    ChainMeta {
        id: "stacks",
        label: "Stacks",
        logo: "/chains/stacks.svg",
        aliases: &["stx", "blockstack"],
        pinned: false,
    },
    ChainMeta {
        id: "polkadot",
        label: "Polkadot",
        logo: "/chains/polkadot.svg",
        aliases: &["dot", "polkadot-relay"],
        pinned: false,
    },
    ChainMeta {
        id: "kava",
        label: "Kava",
        logo: "/chains/kava.svg",
        aliases: &["kava-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "ton",
        label: "TON",
        logo: "/chains/ton.svg",
        // Toncoin token rebranded to Gram (June 2026); chain name stays TON.
        aliases: &[
            "the-open-network",
            "ton-mainnet",
            "gram",
            "gram-token",
            "toncoin",
        ],
        pinned: false,
    },
    ChainMeta {
        id: "taiko",
        label: "Taiko",
        logo: "/chains/taiko.svg",
        aliases: &["taiko-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "immutable",
        label: "Immutable",
        logo: "/chains/immutable.svg",
        aliases: &["imx", "immutablex", "immutable-x", "immutable-zkevm"],
        pinned: false,
    },
    ChainMeta {
        id: "zora",
        label: "Zora",
        logo: "/chains/zora.svg",
        aliases: &["zora-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "stellar",
        label: "Stellar",
        logo: "/chains/stellar.svg",
        aliases: &["xlm", "stellar-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "algorand",
        label: "Algorand",
        logo: "/chains/algorand.svg",
        aliases: &["algo", "algorand-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "filecoin",
        label: "Filecoin",
        logo: "/chains/filecoin.svg",
        aliases: &["fil", "filecoin-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "ronin",
        label: "Ronin",
        logo: "/chains/ronin.svg",
        aliases: &["ron", "ronin-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "worldchain",
        label: "World Chain",
        logo: "/chains/worldchain.svg",
        aliases: &["world-chain", "worldcoin", "wld"],
        pinned: false,
    },
    ChainMeta {
        id: "hedera",
        label: "Hedera",
        logo: "/chains/hedera.svg",
        aliases: &["hbar", "hedera-hashgraph"],
        pinned: false,
    },
    ChainMeta {
        id: "xrpl",
        label: "XRPL",
        logo: "/chains/xrpl.svg",
        aliases: &["xrp", "ripple", "xrpl-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "thorchain",
        label: "THORChain",
        logo: "/chains/thorchain.svg",
        aliases: &["rune", "thorchain-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "katana",
        label: "Katana",
        logo: "/chains/katana.svg",
        aliases: &["katana-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "dydx",
        label: "dYdX",
        logo: "/chains/dydx.svg",
        aliases: &["dydx-chain", "dydx-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "fraxtal",
        label: "Fraxtal",
        logo: "/chains/fraxtal.svg",
        aliases: &["frax", "fraxtal-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "tezos",
        label: "Tezos",
        logo: "/chains/tezos.svg",
        aliases: &["xtz", "tezos-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "mezo",
        label: "Mezo",
        logo: "/chains/mezo.svg",
        aliases: &["mezo-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "bittensor",
        label: "Bittensor",
        logo: "/chains/bittensor.svg",
        aliases: &["tao", "bittensor-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "pulsechain",
        label: "PulseChain",
        logo: "/chains/pulsechain.svg",
        aliases: &["pls", "pulsechain-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "provenance",
        label: "Provenance",
        logo: "/chains/provenance.svg",
        aliases: &["hash", "hash-2", "provenance-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "fluent",
        label: "Fluent",
        logo: "/chains/fluent.svg",
        aliases: &["fluent-network", "fluent-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "hydration",
        label: "Hydration",
        logo: "/chains/hydration.svg",
        aliases: &["hydradx", "hydration-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "mixin",
        label: "Mixin",
        logo: "/chains/mixin.svg",
        aliases: &["xin", "mixin-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "vaulta",
        label: "Vaulta",
        logo: "/chains/vaulta.svg",
        aliases: &["eos", "eosio", "vaulta-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "ethereal",
        label: "Ethereal",
        logo: "/chains/ethereal.svg",
        aliases: &["ethereal-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "stable",
        label: "Stable",
        logo: "/chains/stable.svg",
        aliases: &["stable-2", "stable-mainnet"],
        pinned: false,
    },
    ChainMeta {
        id: "xpr",
        label: "XPR Network",
        logo: "/chains/xpr.svg",
        aliases: &["proton", "xpr-network", "xpr-mainnet"],
        pinned: false,
    },
];

/// Primary-row chain tiles (excluding the All tile).
pub const STRIP_PRIMARY_VISIBLE: usize = 20;

/// DB noise values — not real chains; hidden from card tags and strip counts.
const CHAIN_NOISE: &[&str] = &[
    "all",
    "any",
    "none",
    "unknown",
    "multi-chain",
    "multichain",
    "multi",
    "cross-chain",
    "crosschain",
    "omnichain",
    "omni-chain",
    "ecosystem",
];

/// Strip common network suffixes after separators are normalized.
const CHAIN_SUFFIXES: &[&str] = &["-mainnet", "-testnet", "-network", "-one", "-pos", "-era"];

/// Normalize a raw DB chain string for catalog lookup.
pub fn normalize_chain_key(raw: &str) -> String {
    let mut key = raw.trim().to_lowercase();
    key = key.replace(['_', ' '], "-");
    while key.contains("--") {
        key = key.replace("--", "-");
    }
    key = key.trim_matches('-').to_string();

    loop {
        let mut stripped = false;
        for suffix in CHAIN_SUFFIXES {
            if let Some(base) = key.strip_suffix(suffix) {
                if !base.is_empty() {
                    key = base.to_string();
                    stripped = true;
                }
            }
        }
        if !stripped {
            break;
        }
    }

    key
}

/// Whether a raw DB chain value is catalog noise (not a real chain).
pub fn is_chain_noise(raw: &str) -> bool {
    let key = normalize_chain_key(raw);
    if key.is_empty() {
        return true;
    }
    if CHAIN_NOISE.contains(&key.as_str()) {
        return true;
    }
    key.contains('+') || key.contains("networks")
}

/// Public logo path for a catalog id — always `/chains/{id}.svg`.
pub fn chain_logo_path(id: &str) -> String {
    format!("/chains/{}.svg", id.trim().to_lowercase())
}

/// Resolve a raw DB chain string to a catalog entry, if any.
pub fn resolve_chain(db_value: &str) -> Option<&'static ChainMeta> {
    let normalized = normalize_chain_key(db_value);
    if normalized.is_empty() || is_chain_noise(&normalized) {
        return None;
    }
    CHAIN_CATALOG.iter().find(|entry| {
        entry.id == normalized
            || entry
                .aliases
                .iter()
                .any(|alias| normalize_chain_key(alias) == normalized)
    })
}

/// Lookup by canonical catalog id.
pub fn chain_by_id(id: &str) -> Option<&'static ChainMeta> {
    let normalized = normalize_chain_key(id);
    CHAIN_CATALOG.iter().find(|entry| entry.id == normalized)
}

/// Resolve any raw chain string (id, alias, or variant) to its canonical
/// catalog id. Returns `None` for noise or unrecognized values.
///
/// This is the single entry point for chain synonym normalization —
/// relevance scoring, crawler chain extraction, and UI filter matching
/// all use this to map `bnb` / `bsc` / `binance` → `bsc`, `fantom` / `ftm`
/// → `sonic`, `gram` → `ton`, etc.
pub fn canonical_chain_id(raw: &str) -> Option<&'static str> {
    resolve_chain(raw).map(|meta| meta.id)
}

/// Map `eip155:<chainId>` network strings (x402/Bazaar) to catalog slugs.
fn chain_slug_from_eip155_network(network: &str) -> Option<&'static str> {
    const MAINNET_CHAIN_MAP: &[(u32, &str)] = &[
        (8453, "base"),
        (1, "ethereum"),
        (137, "polygon"),
        (42161, "arbitrum"),
        (10, "optimism"),
        (43114, "avalanche"),
        (56, "bsc"),
        (196, "x-layer"),
    ];
    let rest = network.trim().strip_prefix("eip155:")?;
    let chain_id: u32 = rest.parse().ok()?;
    MAINNET_CHAIN_MAP
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, slug)| *slug)
}

/// Normalize a single chain token (catalog alias, slug, or `eip155:` network).
pub fn normalize_chain_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(id) = canonical_chain_id(trimmed) {
        return Some(id.to_string());
    }
    if let Some(slug) = chain_slug_from_eip155_network(trimmed) {
        return Some(slug.to_string());
    }
    let key = normalize_chain_key(trimmed);
    if key.is_empty() || is_chain_noise(&key) {
        return None;
    }
    Some(key)
}

/// Normalize raw chain strings to canonical catalog ids where possible.
///
/// Maps synonyms (`bnb` → `bsc`, `fantom` → `sonic`), `eip155:` networks, deduplicates,
/// and keeps unrecognized normalized tokens so new chains are not dropped.
pub fn canonicalize_chain_values(raw: &[String]) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for entry in raw {
        let canonical = normalize_chain_token(entry).unwrap_or_else(|| entry.clone());
        if seen.insert(canonical.clone()) {
            out.push(canonical);
        }
    }
    out
}

/// Whether a selected `?chain=` value is active for a catalog entry (id or alias).
pub fn chain_filter_active(entry: &ChainMeta, active: &[String]) -> bool {
    active
        .iter()
        .filter_map(|value| resolve_chain(value))
        .any(|resolved| resolved.id == entry.id)
}

/// Chains for the strip: pinned first (catalog order), then non-pinned by descending count.
pub fn strip_chains(counts: &[(String, i64)]) -> Vec<&'static ChainMeta> {
    use std::collections::HashMap;

    let mut count_map: HashMap<&str, i64> = HashMap::new();
    for (raw, count) in counts {
        if let Some(meta) = resolve_chain(raw) {
            *count_map.entry(meta.id).or_insert(0) += count;
        }
    }

    let mut ordered: Vec<&'static ChainMeta> = Vec::new();

    for entry in CHAIN_CATALOG {
        if entry.pinned {
            ordered.push(entry);
        }
    }

    let mut rest: Vec<&'static ChainMeta> = CHAIN_CATALOG
        .iter()
        .filter(|entry| !entry.pinned)
        .filter(|entry| count_map.get(entry.id).copied().unwrap_or(0) > 0)
        .collect();

    rest.sort_by(|a, b| {
        let ca = count_map.get(a.id).copied().unwrap_or(0);
        let cb = count_map.get(b.id).copied().unwrap_or(0);
        cb.cmp(&ca).then_with(|| a.id.cmp(b.id))
    });

    ordered.extend(rest);
    ordered
}

/// A chain value on a tool card — catalog logo or fallback text pill.
#[derive(Clone, Debug)]
pub struct ChainTagView {
    pub meta: Option<&'static ChainMeta>,
    pub raw: String,
}

/// Map every chain on a tool — no overflow truncation.
pub fn chain_tags_show_all(chains: &[String]) -> (Vec<ChainTagView>, usize) {
    chain_tags_for_tool(chains, chains.len())
}

/// Map tool chain strings to catalog entries; returns visible tags and overflow count.
pub fn chain_tags_for_tool(chains: &[String], max_visible: usize) -> (Vec<ChainTagView>, usize) {
    use std::collections::HashSet;

    let mut tags: Vec<ChainTagView> = Vec::new();
    let mut seen_catalog: HashSet<&'static str> = HashSet::new();
    let mut seen_fallback: HashSet<String> = HashSet::new();

    for raw in chains {
        let trimmed = raw.trim();
        if trimmed.is_empty() || is_chain_noise(trimmed) {
            continue;
        }
        let meta = resolve_chain(trimmed);
        if let Some(entry) = meta {
            if !seen_catalog.insert(entry.id) {
                continue;
            }
        } else {
            let key = normalize_chain_key(trimmed);
            if !seen_fallback.insert(key) {
                continue;
            }
        }
        tags.push(ChainTagView {
            meta,
            raw: trimmed.to_string(),
        });
    }

    let overflow = tags.len().saturating_sub(max_visible);
    let visible = tags.into_iter().take(max_visible).collect();
    (visible, overflow)
}

/// Abbreviated label for unknown chains shown as text pills.
pub fn chain_fallback_label(raw: &str) -> String {
    let token = raw.split(['-', ' ', '_']).next().unwrap_or(raw).trim();
    if token.is_empty() {
        return "?".into();
    }
    let upper = token.to_uppercase();
    if upper.len() <= 5 {
        upper
    } else {
        upper.chars().take(4).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn logo_path_on_disk(logo: &str) -> std::path::PathBuf {
        let file = logo.trim_start_matches('/');
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("public")
            .join(file)
    }

    #[test]
    fn catalog_logo_files_exist() {
        for entry in CHAIN_CATALOG {
            let path = logo_path_on_disk(entry.logo);
            assert!(
                path.exists(),
                "missing logo file for {}: {}",
                entry.id,
                path.display()
            );
        }
    }

    #[derive(serde::Deserialize)]
    struct LogoManifest {
        forbidden: Vec<String>,
        entries: Vec<LogoManifestEntry>,
    }

    #[derive(serde::Deserialize)]
    struct LogoManifestEntry {
        id: String,
        markers: Vec<String>,
        #[serde(default)]
        require_vector: bool,
    }

    fn load_logo_manifest() -> LogoManifest {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("chain-logo-manifest.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read manifest {}: {e}", path.display()));
        serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse manifest {}: {e}", path.display()))
    }

    fn extract_svg_attr(tag: &str, name: &str) -> Option<f64> {
        let needle = format!("{name}=\"");
        let start = tag.find(&needle)?;
        let rest = &tag[start + needle.len()..];
        let end = rest.find('"')?;
        rest[..end].parse().ok()
    }

    #[test]
    fn catalog_logo_tiles_fit_viewbox() {
        const TILE_VB: &str = r#"viewBox="0 0 48 48""#;
        const WRAP_CENTER: &str = "translate(24 24)";
        const MAX_DIRECT: f64 = 48.0;

        for entry in CHAIN_CATALOG {
            let text = std::fs::read_to_string(logo_path_on_disk(entry.logo))
                .unwrap_or_else(|e| panic!("read {}: {e}", entry.logo));
            assert!(text.contains(TILE_VB), "{} missing 48x48 viewBox", entry.id);

            let wrapped = text.contains(WRAP_CENTER);
            for fragment in text.split('<').filter(|s| s.starts_with("rect")) {
                let width = extract_svg_attr(fragment, "width");
                if width == Some(48.0) {
                    continue;
                }
                let x = extract_svg_attr(fragment, "x");
                let y = extract_svg_attr(fragment, "y");
                if let (Some(x), Some(y)) = (x, y) {
                    assert!(
                        x <= MAX_DIRECT && y <= MAX_DIRECT || wrapped,
                        "{}: rect x={x} y={y} outside tile without wrap centering",
                        entry.id
                    );
                }
            }
        }
    }

    #[test]
    fn catalog_logos_use_official_brand_markers() {
        let manifest = load_logo_manifest();
        for entry in &manifest.entries {
            let id = entry.id.as_str();
            let catalog = chain_by_id(id).unwrap_or_else(|| panic!("missing catalog id: {id}"));
            let text = std::fs::read_to_string(logo_path_on_disk(catalog.logo))
                .unwrap_or_else(|e| panic!("read {}: {e}", catalog.logo));
            assert!(
                entry.markers.iter().any(|needle| text.contains(needle)),
                "logo for {id} missing official marker; got head: {}",
                &text[..text.len().min(200)]
            );
            for bad in &manifest.forbidden {
                assert!(
                    !text.contains(bad.as_str()),
                    "logo for {id} contains placeholder/error content: {bad}"
                );
            }
            if text.contains("data:image/png;base64,") {
                let payload = text
                    .split("data:image/png;base64,")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .unwrap_or("");
                assert!(
                    payload.len() > 500,
                    "logo for {id} has truncated/placeholder png embed ({} bytes)",
                    payload.len()
                );
            }
            if entry.require_vector {
                assert!(
                    text.contains("circle") && text.contains("path"),
                    "logo for {id} should be vector circle+path"
                );
                assert!(
                    !text.contains("data:image/png;base64,"),
                    "logo for {id} should not use embedded png"
                );
            }
        }
    }

    #[test]
    fn bitcoin_first_and_pinned() {
        assert_eq!(CHAIN_CATALOG.first().map(|c| c.id), Some("bitcoin"));
        assert!(CHAIN_CATALOG[0].pinned);
    }

    #[test]
    fn bob_pinned() {
        let bob = chain_by_id("bob").expect("bob in catalog");
        assert!(bob.pinned);
    }

    #[test]
    fn catalog_ids_unique() {
        let ids: HashSet<_> = CHAIN_CATALOG.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), CHAIN_CATALOG.len());
    }

    #[test]
    fn catalog_aliases_unique() {
        let mut seen = HashSet::new();
        for entry in CHAIN_CATALOG {
            assert!(seen.insert(entry.id), "duplicate id: {}", entry.id);
            for alias in entry.aliases {
                assert!(
                    seen.insert(*alias),
                    "duplicate alias: {alias} on {}",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn alias_mapping_resolves_common_db_values() {
        let cases = [
            ("eth", "ethereum"),
            ("BTC", "bitcoin"),
            ("matic", "polygon"),
            ("bnb", "bsc"),
            ("BNB Chain", "bsc"),
            ("binance_smart_chain", "bsc"),
            ("arbitrum-one", "arbitrum"),
            ("Arbitrum One", "arbitrum"),
            ("optimism-mainnet", "optimism"),
            ("polygon-pos", "polygon"),
            ("zksync-era", "zksync"),
            ("zk-sync", "zksync"),
            ("avax-c", "avalanche"),
            ("berachain", "bera"),
            ("hyperliquid-xyz", "hyperliquid"),
            ("sei-network", "sei"),
            ("build-on-bitcoin", "bob"),
            ("ETH Mainnet", "ethereum"),
            ("Solana Mainnet", "solana"),
            // Fantom rebranded to Sonic (2025)
            ("fantom", "sonic"),
            ("ftm", "sonic"),
            ("Fantom Mainnet", "sonic"),
            // TON token rebranded to Gram (2026); chain stays TON
            ("gram", "ton"),
            ("gram-token", "ton"),
            ("toncoin", "ton"),
            ("the-open-network", "ton"),
        ];
        for (raw, expected) in cases {
            assert_eq!(
                resolve_chain(raw).map(|c| c.id),
                Some(expected),
                "resolve_chain({raw}) should map to {expected}"
            );
        }
    }

    #[test]
    fn chain_logo_paths_match_catalog_ids() {
        for entry in CHAIN_CATALOG {
            assert_eq!(entry.logo, chain_logo_path(entry.id));
        }
    }

    #[test]
    fn noise_values_map_to_none() {
        for noise in [
            "all",
            "multi-chain",
            "63+ networks",
            "cross-chain",
            "omnichain",
        ] {
            assert!(
                resolve_chain(noise).is_none(),
                "noise value should not resolve: {noise}"
            );
            assert!(is_chain_noise(noise), "noise value not flagged: {noise}");
        }
        for known in ["sonic", "litecoin", "celo", "gnosis", "blast", "okx"] {
            assert!(
                resolve_chain(known).is_some(),
                "catalog chain should resolve: {known}"
            );
        }
        for known_xrpl in ["xrp", "ripple", "xrpl"] {
            assert!(
                resolve_chain(known_xrpl).is_some(),
                "XRPL catalog alias should resolve: {known_xrpl}"
            );
        }
        for unknown in ["anubis", "robinhood"] {
            assert!(
                resolve_chain(unknown).is_none(),
                "unknown chain should not resolve: {unknown}"
            );
            assert!(
                !is_chain_noise(unknown),
                "unknown chain should still render fallback pill: {unknown}"
            );
        }
    }

    #[test]
    fn chain_tags_skip_noise_and_dedupe_catalog_entries() {
        let chains = vec![
            "ethereum".into(),
            "eth".into(),
            "multi-chain".into(),
            "all".into(),
            "base".into(),
        ];
        let (visible, overflow) = chain_tags_for_tool(&chains, 10);
        assert_eq!(visible.len(), 2);
        assert_eq!(overflow, 0);
        assert!(visible.iter().all(|tag| tag.meta.is_some()));
        assert_eq!(visible[0].meta.map(|m| m.id), Some("ethereum"));
        assert_eq!(visible[1].meta.map(|m| m.id), Some("base"));
    }

    #[test]
    fn chain_filter_active_matches_normalized_aliases() {
        let entry = chain_by_id("bsc").expect("bsc");
        let active = vec!["BNB Chain".into(), "ethereum".into()];
        assert!(chain_filter_active(entry, &active));
    }

    #[test]
    fn canonical_chain_id_resolves_synonyms() {
        // BNB Chain synonyms all map to canonical "bsc"
        for raw in &[
            "bnb",
            "bsc",
            "binance",
            "binance-smart-chain",
            "bnb-chain",
            "BNB Chain",
        ] {
            assert_eq!(
                canonical_chain_id(raw),
                Some("bsc"),
                "canonical_chain_id({raw}) should be bsc"
            );
        }
        // Fantom → Sonic rebrand
        for raw in &["fantom", "ftm", "sonic", "fantom-mainnet", "sonic-mainnet"] {
            assert_eq!(
                canonical_chain_id(raw),
                Some("sonic"),
                "canonical_chain_id({raw}) should be sonic"
            );
        }
        // TON token → Gram rebrand (chain stays TON)
        for raw in &["ton", "gram", "gram-token", "toncoin", "the-open-network"] {
            assert_eq!(
                canonical_chain_id(raw),
                Some("ton"),
                "canonical_chain_id({raw}) should be ton"
            );
        }
        // Noise returns None
        assert_eq!(canonical_chain_id("multi-chain"), None);
        assert_eq!(canonical_chain_id("all"), None);
    }

    #[test]
    fn canonicalize_chain_values_maps_synonyms_and_dedupes() {
        let raw = vec![
            "bnb".into(),
            "bsc".into(),
            "Fantom".into(),
            "eip155:8453".into(),
        ];
        let out = canonicalize_chain_values(&raw);
        assert_eq!(out, vec!["bsc", "sonic", "base"]);
    }

    #[test]
    fn normalize_chain_token_handles_eip155_network() {
        assert_eq!(
            normalize_chain_token("eip155:8453"),
            Some("base".to_string())
        );
    }

    #[test]
    fn strip_ordering_pinned_first_then_by_count() {
        let counts = vec![
            ("ethereum".into(), 50),
            ("solana".into(), 30),
            ("base".into(), 10),
        ];
        let ordered = strip_chains(&counts);
        assert_eq!(ordered[0].id, "bitcoin");
        assert_eq!(ordered[1].id, "bob");
        assert_eq!(ordered[2].id, "ethereum");
        assert_eq!(ordered[3].id, "solana");
        assert_eq!(ordered[4].id, "base");
    }

    #[test]
    fn strip_includes_pinned_at_zero_count() {
        let ordered = strip_chains(&[]);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].id, "bitcoin");
        assert_eq!(ordered[1].id, "bob");
    }

    #[test]
    fn strip_primary_visible_leaves_overflow_for_expand_control() {
        assert_eq!(STRIP_PRIMARY_VISIBLE, 20);
        assert_eq!(CHAIN_CATALOG.len(), 73);

        let counts: Vec<(String, i64)> = CHAIN_CATALOG
            .iter()
            .map(|entry| (entry.id.to_string(), 1))
            .collect();
        let ordered = strip_chains(&counts);
        assert_eq!(ordered.len(), CHAIN_CATALOG.len());

        let primary: Vec<_> = ordered
            .iter()
            .take(STRIP_PRIMARY_VISIBLE)
            .copied()
            .collect();
        let overflow: Vec<_> = ordered
            .iter()
            .skip(STRIP_PRIMARY_VISIBLE)
            .copied()
            .collect();
        assert_eq!(primary.len(), STRIP_PRIMARY_VISIBLE);
        assert_eq!(overflow.len(), CHAIN_CATALOG.len() - STRIP_PRIMARY_VISIBLE);
    }

    /// Full BOB Gateway CLI chain union (SDK + live routes); kept in sync with
    /// tests/bob_gateway_registration.rs::bob_gateway_all_chains.
    #[test]
    fn chain_tags_show_all_never_truncates() {
        let chains: Vec<String> = vec!["bitcoin".into(), "bob".into(), "base".into()];
        let (visible, overflow) = chain_tags_show_all(&chains);
        assert_eq!(visible.len(), 3);
        assert_eq!(overflow, 0);
    }

    fn bob_gateway_all_chains() -> Vec<String> {
        vec![
            "bitcoin".into(),
            "bob".into(),
            "ethereum".into(),
            "base".into(),
            "arbitrum".into(),
            "optimism".into(),
            "avalanche".into(),
            "bsc".into(),
            "polygon".into(),
            "sonic".into(),
            "unichain".into(),
            "bera".into(),
            "sei".into(),
            "soneium".into(),
            "tron".into(),
            "hyperliquid".into(),
            "plasma".into(),
        ]
    }

    /// Goal harness: badge resolution for full registered BOB Gateway CLI chains.
    /// Run with `--nocapture` to emit stdout captured in badges.log.
    #[test]
    fn bob_gateway_registered_tool_chain_badges() {
        let registered = bob_gateway_all_chains();
        assert!(registered.len() >= 11, "BOB supports 11+ chains");

        println!(
            "=== resolve_chain (full bob-gateway-cli chains, n={}) ===",
            registered.len()
        );
        let mut catalog_hits = 0usize;
        let mut pill_hits = 0usize;
        for raw in &registered {
            match resolve_chain(raw) {
                Some(meta) => {
                    catalog_hits += 1;
                    let path = logo_path_on_disk(meta.logo);
                    println!(
                        "resolve_chain({raw}) -> id={} label={} logo={} pinned={} file_exists={}",
                        meta.id,
                        meta.label,
                        meta.logo,
                        meta.pinned,
                        path.exists()
                    );
                    if meta.pinned {
                        assert!(path.exists(), "pinned logo file missing for {}", meta.id);
                    }
                }
                None => {
                    pill_hits += 1;
                    println!("resolve_chain({raw}) -> pill (not in catalog, shown as text badge)");
                }
            }
        }
        println!("catalog_logos={catalog_hits} text_pills={pill_hits}");

        for noise in ["multi-chain", "63+ networks", "ecosystem"] {
            assert!(
                resolve_chain(noise).is_none(),
                "noise value must not resolve: {noise}"
            );
            println!("resolve_chain({noise}) -> NONE (noise filtered)");
        }

        let (visible, overflow) = chain_tags_show_all(&registered);
        println!("=== chain_tags_show_all (no truncation) ===");
        println!("visible_count={} overflow={}", visible.len(), overflow);
        for tag in &visible {
            match tag.meta {
                Some(m) => println!("  tag raw={} catalog_id={} logo={}", tag.raw, m.id, m.logo),
                None => println!("  tag raw={} catalog_id=NONE (fallback pill)", tag.raw),
            }
        }

        assert_eq!(visible.len(), registered.len());
        assert_eq!(overflow, 0);
        assert_eq!(
            pill_hits, 0,
            "all BOB chains should resolve to catalog logos"
        );
        assert_eq!(catalog_hits, registered.len());
        assert!(resolve_chain("bitcoin").unwrap().pinned);
        assert!(resolve_chain("bob").unwrap().pinned);
    }
}
