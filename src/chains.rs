//! Chain catalog — allowlist for logo strip and tool-card tags.

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

    #[test]
    fn catalog_logos_use_official_brand_markers() {
        let markers: &[(&str, &[&str])] = &[
            ("bitcoin", &["#f7931a", "#F7931A"]),
            ("ethereum", &["#8a92b2", "#8A92B2", "m959.8"]),
            ("solana", &["#00FFA3", "#9945FF", "linearGradient"]),
            ("base", &["#0052FF"]),
            ("arbitrum", &["#213147", "#12AAFF"]),
            ("optimism", &["#FF0421", "#FF0420"]),
            ("polygon", &["path", "fill"]),
            ("bsc", &["#F0B90B", "#f0b90b"]),
            ("avalanche", &["#FF394A", "#E84142"]),
            ("sui", &["#4DA2FF", "fill-rule"]),
            ("zksync", &["#11141A", "path"]),
            ("bob", &["#F58B00", "#343536"]),
        ];
        let forbidden = [
            "font-size=\"12\"",
            "image content will be provided separately",
            "<!DOCTYPE",
            "404: This page",
            "next-error-h1",
        ];
        for (id, needles) in markers {
            let entry = chain_by_id(id).unwrap_or_else(|| panic!("missing catalog id: {id}"));
            let text = std::fs::read_to_string(logo_path_on_disk(entry.logo))
                .unwrap_or_else(|e| panic!("read {}: {e}", entry.logo));
            assert!(
                needles.iter().any(|needle| text.contains(needle)),
                "logo for {id} missing official marker; got head: {}",
                &text[..text.len().min(200)]
            );
            for bad in forbidden {
                assert!(
                    !text.contains(bad),
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
        }
        let optimism = std::fs::read_to_string(logo_path_on_disk("/chains/optimism.svg")).unwrap();
        assert!(
            optimism.contains("circle") && optimism.contains("path"),
            "optimism logo should be vector circle+path, not raster placeholder"
        );
        assert!(
            !optimism.contains("data:image/png;base64,"),
            "optimism logo should use official vector mark, not embedded png"
        );
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
}
