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

/// Strong onchain/crypto signals — each match adds meaningful score.
const STRONG_SIGNALS: &[(&str, i32, &str)] = &[
    ("bitcoin", 18, "mentions Bitcoin"),
    ("ethereum", 18, "mentions Ethereum"),
    ("solana", 16, "mentions Solana"),
    ("polygon", 14, "mentions Polygon"),
    ("arbitrum", 14, "mentions Arbitrum"),
    ("optimism", 14, "mentions Optimism"),
    ("base chain", 18, "mentions Base"),
    ("base network", 18, "mentions Base"),
    ("avalanche", 14, "mentions Avalanche"),
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

fn add_chain_support(context: &RelevanceContext<'_>, scoring: &mut RelevanceScore) {
    let chains = normalized_chains(context.input.chains);
    for chain in &chains {
        scoring.add_points(14, &format!("supports chain: {chain}"));
    }
    if chains.len() >= 2 && has_wallet_reason(scoring) {
        scoring.add_points(12, "wallet tooling with multi-chain support");
    }
}

fn normalized_chains(chains: &[String]) -> Vec<String> {
    chains
        .iter()
        .map(|chain| chain.to_lowercase())
        .filter(|chain| !chain.is_empty())
        .collect()
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
}
