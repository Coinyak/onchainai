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
        aliases: &["btc"],
        pinned: true,
    },
    ChainMeta {
        id: "bob",
        label: "BOB",
        logo: "/chains/bob.svg",
        aliases: &[],
        pinned: true,
    },
    ChainMeta {
        id: "ethereum",
        label: "Ethereum",
        logo: "/chains/ethereum.svg",
        aliases: &["eth"],
        pinned: false,
    },
    ChainMeta {
        id: "solana",
        label: "Solana",
        logo: "/chains/solana.svg",
        aliases: &["sol"],
        pinned: false,
    },
    ChainMeta {
        id: "base",
        label: "Base",
        logo: "/chains/base.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "arbitrum",
        label: "Arbitrum",
        logo: "/chains/arbitrum.svg",
        aliases: &["arb"],
        pinned: false,
    },
    ChainMeta {
        id: "optimism",
        label: "Optimism",
        logo: "/chains/optimism.svg",
        aliases: &["op"],
        pinned: false,
    },
    ChainMeta {
        id: "polygon",
        label: "Polygon",
        logo: "/chains/polygon.svg",
        aliases: &["matic"],
        pinned: false,
    },
    ChainMeta {
        id: "bsc",
        label: "BNB Chain",
        logo: "/chains/bsc.svg",
        aliases: &["bnb", "binance", "binance-smart-chain"],
        pinned: false,
    },
    ChainMeta {
        id: "avalanche",
        label: "Avalanche",
        logo: "/chains/avalanche.svg",
        aliases: &["avax"],
        pinned: false,
    },
    ChainMeta {
        id: "sui",
        label: "Sui",
        logo: "/chains/sui.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "zksync",
        label: "zkSync",
        logo: "/chains/zksync.svg",
        aliases: &["zk-sync", "zksync-era"],
        pinned: false,
    },
    ChainMeta {
        id: "sonic",
        label: "Sonic",
        logo: "/chains/sonic.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "unichain",
        label: "Unichain",
        logo: "/chains/unichain.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "bera",
        label: "Berachain",
        logo: "/chains/bera.svg",
        aliases: &["berachain"],
        pinned: false,
    },
    ChainMeta {
        id: "sei",
        label: "Sei",
        logo: "/chains/sei.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "soneium",
        label: "Soneium",
        logo: "/chains/soneium.svg",
        aliases: &[],
        pinned: false,
    },
    ChainMeta {
        id: "tron",
        label: "Tron",
        logo: "/chains/tron.svg",
        aliases: &["trx"],
        pinned: false,
    },
    ChainMeta {
        id: "hyperliquid",
        label: "Hyperliquid",
        logo: "/chains/hyperliquid.svg",
        aliases: &["hype"],
        pinned: false,
    },
    ChainMeta {
        id: "plasma",
        label: "Plasma",
        logo: "/chains/plasma.svg",
        aliases: &[],
        pinned: false,
    },
];

/// Primary-row chain tiles (excluding the All tile).
pub const STRIP_PRIMARY_VISIBLE: usize = 6;

/// Resolve a raw DB chain string to a catalog entry, if any.
pub fn resolve_chain(db_value: &str) -> Option<&'static ChainMeta> {
    let normalized = db_value.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    CHAIN_CATALOG.iter().find(|entry| {
        entry.id == normalized
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&normalized))
    })
}

/// Lookup by canonical catalog id.
pub fn chain_by_id(id: &str) -> Option<&'static ChainMeta> {
    let normalized = id.trim().to_lowercase();
    CHAIN_CATALOG.iter().find(|entry| entry.id == normalized)
}

/// Whether a selected `?chain=` value is active for a catalog entry (id or alias).
pub fn chain_filter_active(entry: &ChainMeta, active: &[String]) -> bool {
    active.iter().any(|value| {
        let normalized = value.trim().to_lowercase();
        entry.id == normalized
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&normalized))
    })
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
    let tags: Vec<ChainTagView> = chains
        .iter()
        .map(|raw| ChainTagView {
            meta: resolve_chain(raw),
            raw: raw.clone(),
        })
        .collect();
    let overflow = tags.len().saturating_sub(max_visible);
    let visible = tags.into_iter().take(max_visible).collect();
    (visible, overflow)
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
        assert_eq!(resolve_chain("eth").map(|c| c.id), Some("ethereum"));
        assert_eq!(resolve_chain("BTC").map(|c| c.id), Some("bitcoin"));
        assert_eq!(resolve_chain("matic").map(|c| c.id), Some("polygon"));
        assert_eq!(resolve_chain("bnb").map(|c| c.id), Some("bsc"));
    }

    #[test]
    fn noise_values_map_to_none() {
        for noise in [
            "all",
            "multi-chain",
            "63+ networks",
            "fantom",
            "litecoin",
            "xrp",
            "celo",
            "gnosis",
        ] {
            assert!(
                resolve_chain(noise).is_none(),
                "noise value should not resolve: {noise}"
            );
        }
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

        println!("=== resolve_chain (full bob-gateway-cli chains, n={}) ===", registered.len());
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

        for noise in ["multi-chain", "63+ networks", "fantom"] {
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
                Some(m) => println!(
                    "  tag raw={} catalog_id={} logo={}",
                    tag.raw, m.id, m.logo
                ),
                None => println!("  tag raw={} catalog_id=NONE (fallback pill)", tag.raw),
            }
        }

        assert_eq!(visible.len(), registered.len());
        assert_eq!(overflow, 0);
        assert_eq!(pill_hits, 0, "all BOB chains should resolve to catalog logos");
        assert_eq!(catalog_hits, registered.len());
        assert!(resolve_chain("bitcoin").unwrap().pinned);
        assert!(resolve_chain("bob").unwrap().pinned);
    }
}
