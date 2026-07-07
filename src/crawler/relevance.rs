//! Crypto relevance scanner — scores whether a discovered tool belongs in OnchainAI.
//!
//! Generic MCP/SDK/agent keywords alone must not pass. Strong onchain/crypto
//! evidence (chains, DeFi, wallets, x402, contract tooling) is required for
//! auto-acceptance.

use serde::{Deserialize, Serialize};

/// Output of the crypto relevance scanner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelevanceAssessment {
    pub score: i32,
    pub status: String,
    pub reasons: Vec<String>,
    pub negative_signals: Vec<String>,
}

/// Tool metadata used for relevance scoring (crawler-normalizer input).
#[derive(Debug, Clone)]
pub struct RelevanceInput<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub tool_type: &'a str,
    pub repo_url: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub npm_package: Option<&'a str>,
    pub mcp_endpoint: Option<&'a str>,
    pub chains: &'a [String],
    pub source: &'a str,
    /// Author-declared keywords from package registry metadata (npm, PyPI).
    /// These are explicit intent signals — stronger than free-text scraping
    /// because the author consciously tagged the package. Included in the
    /// scoring corpus so keyword-based signal matching works.
    pub keywords: &'a [String],
}

#[derive(Debug)]
struct RelevanceContext<'a> {
    input: &'a RelevanceInput<'a>,
    text: String,
}

impl<'a> RelevanceContext<'a> {
    fn new(input: &'a RelevanceInput<'a>) -> Self {
        Self {
            input,
            text: corpus(input),
        }
    }

    fn matches(&self, keyword: &str) -> bool {
        matches_keyword(&self.text, keyword)
    }

    fn has_evidence(&self) -> bool {
        input_has_url(self.input.repo_url)
            || input_has_url(self.input.homepage)
            || input_has_value(self.input.npm_package)
            || input_has_url(self.input.mcp_endpoint)
            || !self.input.chains.is_empty()
            || crypto_registry_source(self.input.source)
    }

    fn keyword_hits(&self) -> usize {
        CRYPTO_KEYWORDS
            .iter()
            .filter(|keyword| self.matches(keyword))
            .count()
    }

    fn weak_only_count(&self) -> usize {
        WEAK_ONLY_KEYWORDS
            .iter()
            .filter(|keyword| self.matches(keyword))
            .count()
    }
}

#[derive(Debug, Default)]
struct RelevanceScore {
    score: i32,
    reasons: Vec<String>,
    negative_signals: Vec<String>,
}

impl RelevanceScore {
    fn add_points(&mut self, points: i32, reason: &str) {
        self.score += points;
        self.add_reason(reason);
    }

    fn add_reason(&mut self, reason: &str) {
        if !self.reasons.iter().any(|known| known == reason) {
            self.reasons.push(reason.to_string());
        }
    }

    fn add_negative_signal(&mut self, signal: &str) {
        self.negative_signals.push(signal.to_string());
    }

    fn subtract(&mut self, points: i32) {
        self.score = self.score.saturating_sub(points);
    }

    fn cap_at(&mut self, cap: i32) {
        self.score = self.score.min(cap);
    }

    fn floor_at(&mut self, floor: i32) {
        self.score = self.score.max(floor);
    }

    fn clamp_score(&mut self) {
        self.score = self.score.clamp(0, 100);
    }

    fn has_medium_crypto_signal(&self) -> bool {
        self.reasons.iter().any(|reason| {
            reason.contains("web3")
                || reason.contains("blockchain")
                || reason.contains("crypto")
                || reason.contains("on-chain")
        })
    }

    fn has_strong_or_medium_signal(&self) -> bool {
        self.reasons
            .iter()
            .any(|reason| is_crypto_strength_reason(reason.as_str()))
    }
}

/// Chain tokens that are common English words and prone to false positives
/// when matched as bare tokens in the corpus. For these, require a phrase
/// match (e.g. "ton blockchain", "gram network") instead of bare-token
/// matching.
///
/// `chains[]` (author-declared) is unaffected — explicit chain declarations
/// are always trusted. This only governs corpus text scanning and
/// keyword-based chain extraction (npm/pypi `is_chain_keyword`).
const AMBIGUOUS_CHAIN_TOKENS: &[&str] = &[
    "ton",    // common word (metric ton, tone)
    "gram",   // common unit
    "near",   // common preposition
    "sei",    // Japanese word
    "ink",    // common word
    "flare",  // common word
    "stable", // common adjective
    "stacks", // common word
    "move",   // common verb (Movement chain)
];

/// Phrases that disambiguate ambiguous chain tokens. If any of these
/// phrases appear in the corpus, the chain signal is awarded.
fn ambiguous_chain_phrases(token: &str) -> &'static [&'static str] {
    match token {
        "ton" => &[
            "ton blockchain",
            "ton network",
            "the open network",
            "toncoin",
            "gram token",
        ],
        "gram" => &["gram token", "gram blockchain", "gram network", "toncoin"],
        "near" => &[
            "near protocol",
            "near blockchain",
            "near network",
            "near chain",
        ],
        "sei" => &["sei network", "sei blockchain", "sei chain", "sei-mainnet"],
        "ink" => &["ink chain", "inkchain", "ink-mainnet", "ink blockchain"],
        "flare" => &[
            "flare network",
            "flare blockchain",
            "flare chain",
            "flare-mainnet",
        ],
        "stable" => &["stable chain", "stable-mainnet", "stable blockchain"],
        "stacks" => &["stacks blockchain", "stacks chain", "blockstack", "stx"],
        "move" => &[
            "movement",
            "move chain",
            "movement-mainnet",
            "movement-labs",
        ],
        _ => &[],
    }
}

/// Check if a chain token is ambiguous and, if so, require a phrase match.
fn chain_token_matches(context: &RelevanceContext<'_>, token: &str) -> bool {
    if AMBIGUOUS_CHAIN_TOKENS.contains(&token) {
        // For ambiguous tokens, require a disambiguating phrase.
        ambiguous_chain_phrases(token)
            .iter()
            .any(|phrase| context.matches(phrase))
    } else {
        context.matches(token)
    }
}

/// Check whether a keyword is a recognized chain name **and** safe to
/// extract from registry keywords.
///
/// Delegates to [`crate::chains::canonical_chain_id`] for chain recognition,
/// but excludes ambiguous tokens (ton, gram, near, etc.) that are common
/// English words. A package tagging `keywords: ["ton"]` might mean "a ton
/// of features" — we don't want that in `chains[]`.
///
/// For ambiguous tokens, the caller should rely on explicit `chains[]`
/// declarations (user submissions) or phrase-matched corpus signals
/// ([`add_canonical_chain_signals`]) instead.
pub fn is_chain_keyword(keyword: &str) -> bool {
    if AMBIGUOUS_CHAIN_TOKENS.contains(&keyword) {
        return false;
    }
    crate::chains::canonical_chain_id(keyword).is_some()
}

/// Tiered points for a chain, based on its canonical catalog id.
///
/// Flagship L1s get 18, major L1s get 16, everything else gets 14.
/// Both `add_chain_support` (author-declared `chains[]`) and corpus text
/// matching use this so a chain scores the same regardless of which
/// synonym name the author used.
fn chain_tier_points(canonical_id: &str) -> i32 {
    match canonical_id {
        "bitcoin" | "ethereum" | "base" => 18,
        "solana" | "bsc" | "cosmos" | "near" | "sui" | "aptos" | "tron" | "hyperliquid" => 16,
        _ => 14,
    }
}

/// Strong onchain/crypto signals — each match adds meaningful score.
///
/// Chain signals are handled separately by [`add_chain_support`] and
/// [`add_canonical_chain_signals`] which use `chains::canonical_chain_id`
/// for synonym normalization. Only non-chain strong signals are listed here.
const STRONG_SIGNALS: &[(&str, i32, &str)] = &[
    ("defi", 16, "DeFi keyword"),
    ("uniswap", 16, "DeFi protocol (Uniswap)"),
    ("aave", 16, "DeFi protocol (Aave)"),
    ("wallet", 18, "wallet/custody tooling"),
    ("custody", 16, "custody tooling"),
    ("bridge", 16, "cross-chain bridge"),
    ("cross-chain", 16, "cross-chain tooling"),
    ("bob gateway", 20, "BOB Gateway bridge tooling"),
    ("wormhole", 16, "Wormhole bridge"),
    ("layerzero", 16, "LayerZero bridge"),
    ("x402", 18, "x402 payments"),
    ("rwa", 16, "real-world assets"),
    ("smart contract", 16, "smart contract tooling"),
    ("hardhat", 14, "contract dev tooling"),
    ("foundry", 14, "contract dev tooling"),
    ("web3-mcp", 18, "crypto MCP registry source"),
    ("crypto-mcp", 18, "crypto MCP topic"),
    ("blockchain-mcp", 18, "blockchain MCP topic"),
    ("onchain", 16, "onchain keyword"),
    ("sign transaction", 14, "transaction signing"),
    ("dex", 14, "DEX tooling"),
    ("staking", 14, "staking tooling"),
    ("nft", 12, "NFT tooling"),
    ("token", 10, "token tooling"),
    ("evm", 16, "EVM tooling"),
];

/// Medium signals — helpful but not sufficient alone.
const MEDIUM_SIGNALS: &[(&str, i32, &str)] = &[
    ("web3", 12, "web3 keyword"),
    ("blockchain", 12, "blockchain keyword"),
    ("crypto", 10, "crypto keyword"),
    ("on-chain", 12, "on-chain keyword"),
    ("oracle", 10, "oracle tooling"),
    ("indexer", 10, "indexer tooling"),
    ("rpc", 8, "RPC tooling"),
    ("mcp", 4, "MCP surface (weak alone)"),
    ("sdk", 4, "SDK surface (weak alone)"),
    ("agent", 4, "agent surface (weak alone)"),
    ("gateway", 4, "gateway surface (weak alone)"),
    ("api", 3, "API surface (weak alone)"),
];

/// Hard reject categories — non-crypto productivity MCP/tools.
const REJECT_CATEGORIES: &[(&str, &str)] = &[
    ("filesystem", "generic filesystem MCP"),
    ("file system", "generic filesystem MCP"),
    ("weather", "weather MCP"),
    ("calendar", "calendar MCP"),
    ("todo", "productivity MCP"),
    ("notes", "productivity MCP"),
    ("email client", "productivity MCP"),
    ("spreadsheet", "productivity MCP"),
];

const WEAK_ONLY_KEYWORDS: &[&str] = &["mcp", "sdk", "agent", "gateway", "api"];

const CRYPTO_KEYWORDS: &[&str] = &[
    "web3",
    "defi",
    "wallet",
    "bridge",
    "bitcoin",
    "ethereum",
    "solana",
    "crypto",
    "blockchain",
    "token",
    "nft",
    "staking",
    "dex",
    "onchain",
    "x402",
    "rwa",
    "evm",
];

fn corpus(input: &RelevanceInput<'_>) -> String {
    let mut parts = vec![input.name.to_lowercase()];
    if let Some(d) = input.description {
        parts.push(d.to_lowercase());
    }
    if let Some(r) = input.repo_url {
        parts.push(r.to_lowercase());
    }
    if let Some(p) = input.npm_package {
        parts.push(p.to_lowercase());
    }
    // Author-declared keywords are explicit intent signals — include them
    // so signal matching can score on the full metadata surface.
    for kw in input.keywords {
        parts.push(kw.to_lowercase());
    }
    parts.join(" ")
}

fn input_has_url(value: Option<&str>) -> bool {
    value.is_some_and(|url| !url.trim().is_empty() && url.starts_with("http"))
}

fn input_has_value(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn crypto_registry_source(source: &str) -> bool {
    matches!(source, "cryptoskill" | "web3-mcp-hub")
}

fn matches_keyword(text: &str, keyword: &str) -> bool {
    if phrase_keyword(keyword) {
        text.contains(keyword)
    } else {
        text.split(|c: char| !c.is_alphanumeric())
            .any(|word| word == keyword)
    }
}

fn phrase_keyword(keyword: &str) -> bool {
    keyword.contains([' ', '.', '-'])
}

fn is_crypto_strength_reason(reason: &str) -> bool {
    !GENERIC_REASON_FRAGMENTS
        .iter()
        .any(|fragment| reason.contains(fragment))
}

const GENERIC_REASON_FRAGMENTS: &[&str] = &[
    "weak alone",
    "MCP surface",
    "SDK surface",
    "agent surface",
    "gateway surface",
    "API surface",
    "trustworthy listing evidence",
];

/// Assess crypto relevance for a crawled or submitted tool.
pub fn assess_relevance(input: &RelevanceInput<'_>) -> RelevanceAssessment {
    let context = RelevanceContext::new(input);
    let mut scoring = RelevanceScore::default();

    add_signal_matches(&context, &mut scoring, STRONG_SIGNALS);
    add_signal_matches(&context, &mut scoring, MEDIUM_SIGNALS);
    add_canonical_chain_signals(&context, &mut scoring);
    add_chain_support(&context, &mut scoring);
    add_package_name_signal(&context, &mut scoring);
    add_repo_url_signal(&context, &mut scoring);
    add_source_signal(&context, &mut scoring);
    apply_evidence_gate(&context, &mut scoring);
    apply_reject_categories(&context, &mut scoring);
    apply_generic_surface_gate(&context, &mut scoring);
    apply_keyword_stuffing_gate(&context, &mut scoring);
    scoring.clamp_score();

    RelevanceAssessment {
        score: scoring.score,
        status: status_for(&scoring).to_string(),
        reasons: scoring.reasons,
        negative_signals: scoring.negative_signals,
    }
}

fn add_signal_matches(
    context: &RelevanceContext<'_>,
    scoring: &mut RelevanceScore,
    signals: &[(&str, i32, &str)],
) {
    for (keyword, points, reason) in signals {
        if context.matches(keyword) {
            scoring.add_points(*points, reason);
        }
    }
}

/// Score chain signals from the corpus text (name/description/keywords).
///
/// Scans the corpus for every catalog chain id and alias, resolves each
/// match to its canonical id, and awards tiered points. Deduplicates by
/// canonical id so `bnb` + `bsc` in the same text = one chain signal,
/// not two.
///
/// Ambiguous tokens (ton, gram, near, etc.) require a disambiguating phrase
/// to avoid false positives from common English words.
fn add_canonical_chain_signals(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    use std::collections::HashSet;

    let mut seen_canonical: HashSet<&str> = HashSet::new();
    for entry in crate::chains::CHAIN_CATALOG {
        // Check the canonical id itself (with ambiguous-token guard).
        if chain_token_matches(context, entry.id) {
            if seen_canonical.insert(entry.id) {
                let pts = chain_tier_points(entry.id);
                scoring.add_points(pts, &format!("mentions {}", entry.label));
            }
            continue;
        }
        // Check aliases — if matched, resolve to the canonical id.
        for alias in entry.aliases {
            if chain_token_matches(context, alias) {
                if seen_canonical.insert(entry.id) {
                    let pts = chain_tier_points(entry.id);
                    scoring.add_points(pts, &format!("mentions {}", entry.label));
                }
                break;
            }
        }
    }
}

fn add_chain_support(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    use std::collections::HashSet;

    let mut seen_canonical: HashSet<&str> = HashSet::new();
    for raw in context.input.chains {
        if let Some(canonical) = crate::chains::canonical_chain_id(raw) {
            if seen_canonical.insert(canonical) {
                let pts = chain_tier_points(canonical);
                scoring.add_points(pts, &format!("supports chain: {canonical}"));
            }
        }
    }
    if seen_canonical.len() >= 2 && has_wallet_reason(scoring) {
        scoring.add_points(12, "wallet tooling with multi-chain support");
    }
}

fn has_wallet_reason(scoring: &RelevanceScore) -> bool {
    scoring
        .reasons
        .iter()
        .any(|reason| reason.contains("wallet"))
}

fn add_package_name_signal(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if context.input.npm_package.is_some_and(crypto_named_package) {
        scoring.add_points(8, "npm package with crypto naming");
    }
}

fn crypto_named_package(package: &str) -> bool {
    package.contains('@') || package.contains("web3") || package.contains("crypto")
}

fn add_repo_url_signal(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if context.input.repo_url.is_some_and(crypto_named_github_url) {
        scoring.add_points(8, "repo URL suggests crypto project");
    }
}

fn crypto_named_github_url(url: &str) -> bool {
    url.contains("github.com")
        && CRYPTO_REPO_MARKERS
            .iter()
            .any(|marker| url.contains(marker))
}

const CRYPTO_REPO_MARKERS: &[&str] = &["web3", "defi", "crypto", "wallet", "chain", "bob"];

fn add_source_signal(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if crypto_registry_source(context.input.source) {
        scoring.add_points(
            15,
            &format!("discovered via crypto registry ({})", context.input.source),
        );
    }
}

fn apply_evidence_gate(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if context.has_evidence() {
        scoring.add_points(5, "has trustworthy listing evidence");
        return;
    }

    scoring.add_negative_signal("sparse evidence (no repo/homepage/npm/MCP endpoint)");
    if scoring.has_medium_crypto_signal() {
        scoring.floor_at(42);
    } else {
        scoring.subtract(10);
    }
}

fn apply_reject_categories(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    for (keyword, label) in REJECT_CATEGORIES {
        if context.matches(keyword) {
            scoring.add_negative_signal(label);
            scoring.subtract(35);
        }
    }
}

fn apply_generic_surface_gate(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if context.weak_only_count() == 0 {
        return;
    }
    if scoring.has_strong_or_medium_signal() || !context.input.chains.is_empty() {
        return;
    }

    scoring.add_negative_signal("only generic MCP/SDK/agent/gateway/api keywords");
    scoring.cap_at(25);
}

fn apply_keyword_stuffing_gate(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    if context.keyword_hits() >= 4 && !context.has_evidence() {
        scoring.add_negative_signal("keyword stuffing without listing evidence");
        scoring.cap_at(55);
    }
}

fn status_for(scoring: &RelevanceScore) -> &'static str {
    if should_reject_for_negative_signal(scoring) {
        "rejected"
    } else if scoring.score >= 70 {
        "accepted"
    } else if scoring.score >= 40 {
        "needs_review"
    } else {
        "rejected"
    }
}

fn should_reject_for_negative_signal(scoring: &RelevanceScore) -> bool {
    scoring.score < 50
        && scoring.negative_signals.iter().any(|signal| {
            BLOCKING_NEGATIVE_SIGNALS
                .iter()
                .any(|label| signal.contains(label))
        })
}

const BLOCKING_NEGATIVE_SIGNALS: &[&str] = &["filesystem", "weather", "calendar"];

#[cfg(test)]
mod tests {
    use super::*;

    struct TestInput<'a> {
        name: &'a str,
        description: Option<&'a str>,
        chains: &'a [String],
        repo: Option<&'a str>,
        npm: Option<&'a str>,
    }

    impl<'a> TestInput<'a> {
        fn new(name: &'a str, description: Option<&'a str>) -> Self {
            Self {
                name,
                description,
                chains: &[],
                repo: None,
                npm: None,
            }
        }

        fn chains(mut self, chains: &'a [String]) -> Self {
            self.chains = chains;
            self
        }

        fn repo(mut self, repo: &'a str) -> Self {
            self.repo = Some(repo);
            self
        }

        fn npm(mut self, npm: &'a str) -> Self {
            self.npm = Some(npm);
            self
        }

        fn build(self) -> RelevanceInput<'a> {
            RelevanceInput {
                name: self.name,
                description: self.description,
                tool_type: "mcp",
                repo_url: self.repo,
                homepage: None,
                npm_package: self.npm,
                mcp_endpoint: None,
                chains: self.chains,
                source: "github",
                keywords: &[],
            }
        }
    }

    fn assess(test_input: TestInput<'_>) -> RelevanceAssessment {
        assess_relevance(&test_input.build())
    }

    #[test]
    fn accepted_bob_gateway_cli() {
        let chains = vec!["bitcoin".into(), "ethereum".into(), "base".into()];
        let assessment = assess(
            TestInput::new(
                "BOB Gateway CLI",
                Some("Bitcoin to EVM bridge CLI for cross-chain transfers"),
            )
            .chains(&chains)
            .repo("https://github.com/bob-collective/bob")
            .npm("@gobob/gateway-cli"),
        );
        assert_eq!(assessment.status, "accepted");
        assert!(assessment.score >= 70);
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.contains("Bitcoin") || r.contains("bridge") || r.contains("EVM")));
    }

    #[test]
    fn accepted_wallet_mcp_with_chains() {
        let chains = vec!["ethereum".into(), "polygon".into()];
        let assessment = assess(
            TestInput::new(
                "Chain Wallet MCP",
                Some("MCP server for wallet operations and chain signing"),
            )
            .chains(&chains)
            .repo("https://github.com/example/wallet-mcp"),
        );
        assert_eq!(assessment.status, "accepted");
        assert!(assessment.score >= 70);
        assert!(assessment.reasons.iter().any(|r| r.contains("wallet")));
    }

    #[test]
    fn needs_review_generic_web3_helper_sparse_evidence() {
        let assessment = assess(TestInput::new(
            "Web3 Helper",
            Some("A generic web3 helper utility"),
        ));
        assert_eq!(assessment.status, "needs_review");
        assert!(assessment.score >= 40 && assessment.score < 70);
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("sparse evidence")));
    }

    #[test]
    fn rejected_filesystem_mcp_only() {
        let assessment = assess(TestInput::new(
            "Filesystem MCP",
            Some("MCP server for local file operations"),
        ));
        assert_eq!(assessment.status, "rejected");
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("filesystem")));
    }

    #[test]
    fn rejected_weather_calendar_mcp() {
        let weather = assess(TestInput::new(
            "Weather MCP",
            Some("MCP server for weather forecasts"),
        ));
        assert_eq!(weather.status, "rejected");
        assert!(weather
            .negative_signals
            .iter()
            .any(|s| s.contains("weather")));

        let calendar = assess(TestInput::new(
            "Calendar MCP",
            Some("MCP integration for calendar events"),
        ));
        assert_eq!(calendar.status, "rejected");
        assert!(calendar
            .negative_signals
            .iter()
            .any(|s| s.contains("calendar")));
    }

    #[test]
    fn keyword_stuffing_without_evidence_needs_review_or_rejected() {
        let assessment = assess(TestInput::new(
            "Crypto Mega Tool",
            Some(
                "web3 defi wallet bridge bitcoin ethereum solana crypto blockchain token nft staking dex onchain x402 rwa evm",
            ),
        ));
        assert!(
            assessment.status == "needs_review" || assessment.status == "rejected",
            "expected needs_review or rejected, got {}",
            assessment.status
        );
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("keyword stuffing")));
    }

    #[test]
    fn generic_mcp_alone_does_not_auto_accept() {
        let assessment = assess(TestInput::new("Generic MCP", Some("An MCP server")));
        assert_ne!(assessment.status, "accepted");
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("generic") || s.contains("sparse")));
    }

    #[test]
    fn cryptoskill_source_boosts_borderline_tool() {
        let chains = vec!["ethereum".into()];
        let mut inp = TestInput::new("Swap Router SDK", Some("SDK for DEX routing on EVM chains"))
            .chains(&chains)
            .repo("https://github.com/example/swap-router")
            .npm("@example/swap-router")
            .build();
        inp.source = "cryptoskill";
        let assessment = assess_relevance(&inp);
        assert!(assessment.score >= 40);
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.contains("crypto registry")));
    }

    #[test]
    fn accepted_base_network_wallet_agent() {
        let assessment = assess(
            TestInput::new(
                "Base Wallet Agent MCP",
                Some("Onchain wallet operations for Base network agents"),
            )
            .repo("https://github.com/example/base-wallet-agent")
            .npm("@example/base-wallet-agent"),
        );
        assert_eq!(assessment.status, "accepted");
        assert!(assessment.reasons.iter().any(|r| r.contains("Base")));
    }

    #[test]
    fn rejects_generic_indexing_without_dex_word() {
        let assessment = assess(
            TestInput::new(
                "codebase-memory-mcp",
                Some("Indexes codebases into a persistent knowledge graph for AI coding agents"),
            )
            .repo("https://github.com/example/codebase-memory-mcp"),
        );
        assert_ne!(assessment.status, "accepted");
        assert!(!assessment.reasons.iter().any(|r| r.contains("DEX")));
    }

    #[test]
    fn rejects_cryptographic_identity_without_onchain_signal() {
        let assessment = assess(
            TestInput::new(
                "osaurus",
                Some("Native macOS harness for AI agents with cryptographic identity"),
            )
            .repo("https://github.com/example/osaurus"),
        );
        assert_ne!(assessment.status, "accepted");
        assert!(!assessment.reasons.iter().any(|r| r == "crypto keyword"));
    }

    #[test]
    fn repo_url_in_corpus_affects_keyword_matching() {
        let without_repo = assess(TestInput::new(
            "Gateway Tool",
            Some("A transfer utility for agents"),
        ));
        let with_repo = assess(
            TestInput::new("Gateway Tool", Some("A transfer utility for agents"))
                .repo("https://github.com/example/ethereum-bridge"),
        );
        assert!(with_repo.score > without_repo.score);
        assert!(with_repo.reasons.iter().any(|r| r.contains("Ethereum")));
        assert!(!without_repo.reasons.iter().any(|r| r.contains("Ethereum")));
    }

    #[test]
    fn accepted_bnb_agent_sdk_with_keywords() {
        // Simulates the bnbagent PyPI package: keywords declare bnb, web3,
        // blockchain, ethereum, x402 but the description is short.
        // Before the fix, keywords were excluded from the corpus and the
        // score was too low for acceptance.
        let inp = RelevanceInput {
            name: "bnbagent",
            description: Some("Modular Python SDK for on-chain AI agents on BNB Chain"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/bnb-chain/bnbagent-sdk"),
            homepage: Some("https://github.com/bnb-chain/bnbagent-sdk"),
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into(), "bsc".into(), "ethereum".into()],
            source: "pypi",
            keywords: &[
                "agent".into(),
                "binance-smart-chain".into(),
                "blockchain".into(),
                "bsc".into(),
                "erc-8004".into(),
                "erc-8183".into(),
                "ethereum".into(),
                "modular".into(),
                "sdk".into(),
                "web3".into(),
            ],
        };
        let assessment = assess_relevance(&inp);
        assert_eq!(
            assessment.status, "accepted",
            "bnbagent should be accepted, got {} (score {})",
            assessment.status, assessment.score
        );
        assert!(assessment.score >= 70);
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.contains("BNB") || r.contains("Binance") || r.contains("BSC")));
    }

    #[test]
    fn accepted_bnbagent_studio_cli_with_keywords() {
        // Simulates bnbagent-studio: a CLI with x402 and bnb keywords.
        let inp = RelevanceInput {
            name: "bnbagent-studio",
            description: Some(
                "The bag CLI to scaffold and deploy a bnbagent-sdk seller agent on BNB Chain",
            ),
            tool_type: "cli",
            repo_url: Some("https://github.com/bnb-chain/bnbagent-studio"),
            homepage: Some("https://github.com/bnb-chain/bnbagent-studio"),
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into()],
            source: "pypi",
            keywords: &[
                "agent".into(),
                "blockchain".into(),
                "bnb".into(),
                "cli".into(),
                "erc-8004".into(),
                "erc-8183".into(),
                "x402".into(),
            ],
        };
        let assessment = assess_relevance(&inp);
        assert_eq!(
            assessment.status, "accepted",
            "bnbagent-studio should be accepted, got {} (score {})",
            assessment.status, assessment.score
        );
        assert!(assessment.score >= 70);
    }

    #[test]
    fn keywords_in_corpus_boost_score_vs_without() {
        // Same tool, but one has keywords in the metadata and the other doesn't.
        let without_keywords = RelevanceInput {
            name: "bnbagent",
            description: Some("Modular Python SDK for on-chain AI agents on BNB Chain"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/bnb-chain/bnbagent-sdk"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into()],
            source: "pypi",
            keywords: &[],
        };
        let with_keywords = RelevanceInput {
            name: "bnbagent",
            description: Some("Modular Python SDK for on-chain AI agents on BNB Chain"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/bnb-chain/bnbagent-sdk"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into()],
            source: "pypi",
            keywords: &[
                "blockchain".into(),
                "bsc".into(),
                "ethereum".into(),
                "web3".into(),
                "x402".into(),
            ],
        };
        let without = assess_relevance(&without_keywords);
        let with_kw = assess_relevance(&with_keywords);
        assert!(
            with_kw.score > without.score,
            "keywords should boost score: with={} vs without={}",
            with_kw.score,
            without.score
        );
    }

    #[test]
    fn chain_synonyms_resolve_to_same_canonical_id() {
        // BNB Chain synonyms all resolve to "bsc" in the chain catalog.
        for raw in &["bnb", "bsc", "binance", "binance-smart-chain", "bnb-chain"] {
            assert_eq!(
                crate::chains::canonical_chain_id(raw),
                Some("bsc"),
                "{raw} should resolve to bsc"
            );
        }
        // Fantom rebranded to Sonic.
        for raw in &["fantom", "ftm", "sonic"] {
            assert_eq!(
                crate::chains::canonical_chain_id(raw),
                Some("sonic"),
                "{raw} should resolve to sonic"
            );
        }
        // TON token rebranded to Gram; chain stays TON.
        for raw in &["ton", "gram", "toncoin"] {
            assert_eq!(
                crate::chains::canonical_chain_id(raw),
                Some("ton"),
                "{raw} should resolve to ton"
            );
        }
    }

    #[test]
    fn chain_synonyms_dont_double_score() {
        // A tool mentioning both "bnb" and "bsc" should score the same as
        // mentioning only one — they're the same chain.
        let both = RelevanceInput {
            name: "test",
            description: Some("bnb bsc tool"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/example/test"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into(), "bsc".into()],
            source: "pypi",
            keywords: &[],
        };
        let one = RelevanceInput {
            name: "test",
            description: Some("bnb tool"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/example/test"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &["bnb".into()],
            source: "pypi",
            keywords: &[],
        };
        let score_both = assess_relevance(&both).score;
        let score_one = assess_relevance(&one).score;
        assert_eq!(
            score_both, score_one,
            "bnb+bsc should not double-score (both={score_both}, one={score_one})"
        );
    }

    #[test]
    fn ambiguous_token_ton_does_not_match_bare_word() {
        // "ton" as a bare word (e.g. "a ton of features") should NOT
        // trigger a TON chain signal.
        let bare = RelevanceInput {
            name: "heavy-loader",
            description: Some("Process a ton of data files efficiently"),
            tool_type: "cli",
            repo_url: Some("https://github.com/example/loader"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &[],
            source: "github",
            keywords: &[],
        };
        let assessment = assess_relevance(&bare);
        assert!(
            !assessment.reasons.iter().any(|r| r.contains("TON")),
            "bare 'ton' should not trigger TON chain signal: {assessment:?}"
        );
    }

    #[test]
    fn ambiguous_token_ton_matches_with_phrase_context() {
        // "ton blockchain" or "toncoin" should trigger TON chain signal.
        let with_context = RelevanceInput {
            name: "ton-wallet",
            description: Some("Wallet for the TON blockchain ecosystem"),
            tool_type: "mcp",
            repo_url: Some("https://github.com/example/ton-wallet"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &[],
            source: "github",
            keywords: &[],
        };
        let assessment = assess_relevance(&with_context);
        assert!(
            assessment.reasons.iter().any(|r| r.contains("TON")),
            "'TON blockchain' should trigger TON chain signal: {assessment:?}"
        );
    }

    #[test]
    fn ambiguous_token_near_does_not_match_bare_word() {
        // "near" as a preposition should NOT trigger NEAR chain signal.
        let bare = RelevanceInput {
            name: "quick-cache",
            description: Some("Fast cache for data stored near the client"),
            tool_type: "sdk",
            repo_url: Some("https://github.com/example/cache"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &[],
            source: "github",
            keywords: &[],
        };
        let assessment = assess_relevance(&bare);
        assert!(
            !assessment.reasons.iter().any(|r| r.contains("NEAR")),
            "bare 'near' should not trigger NEAR chain signal: {assessment:?}"
        );
    }

    #[test]
    fn explicit_chain_declaration_overrides_ambiguity() {
        // Even ambiguous tokens in `chains[]` (author-declared) are trusted.
        let inp = RelevanceInput {
            name: "gram-bridge",
            description: Some("Bridge tool for GRAM token"),
            tool_type: "cli",
            repo_url: Some("https://github.com/example/gram-bridge"),
            homepage: None,
            npm_package: None,
            mcp_endpoint: None,
            chains: &["ton".into()],
            source: "github",
            keywords: &[],
        };
        let assessment = assess_relevance(&inp);
        assert!(
            assessment
                .reasons
                .iter()
                .any(|r| r.contains("supports chain: ton")),
            "explicit chains[] should always be trusted: {assessment:?}"
        );
    }
}
