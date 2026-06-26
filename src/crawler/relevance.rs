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

/// Strong onchain/crypto signals — each match adds meaningful score.
const STRONG_SIGNALS: &[(&str, i32, &str)] = &[
    ("bitcoin", 18, "mentions Bitcoin"),
    ("ethereum", 18, "mentions Ethereum"),
    ("solana", 16, "mentions Solana"),
    ("polygon", 14, "mentions Polygon"),
    ("arbitrum", 14, "mentions Arbitrum"),
    ("optimism", 14, "mentions Optimism"),
    ("base chain", 14, "mentions Base"),
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

fn corpus(input: &RelevanceInput<'_>) -> String {
    let mut parts = vec![input.name.to_lowercase()];
    if let Some(d) = input.description {
        parts.push(d.to_lowercase());
    }
    if let Some(p) = input.npm_package {
        parts.push(p.to_lowercase());
    }
    parts.join(" ")
}

fn has_evidence(input: &RelevanceInput<'_>) -> bool {
    let has_url = input
        .repo_url
        .is_some_and(|u| !u.trim().is_empty() && u.starts_with("http"));
    let has_homepage = input
        .homepage
        .is_some_and(|u| !u.trim().is_empty() && u.starts_with("http"));
    let has_npm = input.npm_package.is_some_and(|p| !p.trim().is_empty());
    let has_mcp = input
        .mcp_endpoint
        .is_some_and(|u| !u.trim().is_empty() && u.starts_with("http"));
    let has_chains = !input.chains.is_empty();
    let crypto_registry_source = matches!(input.source, "cryptoskill" | "web3-mcp-hub");

    has_url || has_homepage || has_npm || has_mcp || has_chains || crypto_registry_source
}

fn count_weak_only_signals(text: &str) -> usize {
    ["mcp", "sdk", "agent", "gateway", "api"]
        .iter()
        .filter(|k| matches_keyword(text, k))
        .count()
}

fn matches_keyword(text: &str, keyword: &str) -> bool {
    if keyword.contains(' ') || keyword.contains('.') || keyword.contains('-') {
        text.contains(keyword)
    } else {
        text.split(|c: char| !c.is_alphanumeric())
            .any(|w| w == keyword)
    }
}

fn count_crypto_keyword_hits(text: &str) -> usize {
    let keywords = [
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
    keywords.iter().filter(|k| matches_keyword(text, k)).count()
}

/// Assess crypto relevance for a crawled or submitted tool.
pub fn assess_relevance(input: &RelevanceInput<'_>) -> RelevanceAssessment {
    let text = corpus(input);
    let mut score: i32 = 0;
    let mut reasons: Vec<String> = Vec::new();
    let mut negative_signals: Vec<String> = Vec::new();

    for (keyword, points, reason) in STRONG_SIGNALS {
        if matches_keyword(&text, keyword) {
            score += points;
            if !reasons.iter().any(|r| r == reason) {
                reasons.push((*reason).to_string());
            }
        }
    }

    for (keyword, points, reason) in MEDIUM_SIGNALS {
        if matches_keyword(&text, keyword) {
            score += points;
            if !reasons.iter().any(|r| r == reason) {
                reasons.push((*reason).to_string());
            }
        }
    }

    let mut chain_count = 0usize;
    for chain in input.chains {
        let c = chain.to_lowercase();
        if !c.is_empty() {
            chain_count += 1;
            score += 14;
            let reason = format!("supports chain: {c}");
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
        }
    }

    if chain_count >= 2 && reasons.iter().any(|r| r.contains("wallet")) {
        score += 12;
        reasons.push("wallet tooling with multi-chain support".into());
    }

    if input
        .npm_package
        .is_some_and(|p| p.contains('@') || p.contains("web3") || p.contains("crypto"))
    {
        score += 8;
        reasons.push("npm package with crypto naming".into());
    }

    if input.repo_url.is_some_and(|u| {
        u.contains("github.com")
            && (u.contains("web3")
                || u.contains("defi")
                || u.contains("crypto")
                || u.contains("wallet")
                || u.contains("chain")
                || u.contains("bob"))
    }) {
        score += 8;
        reasons.push("repo URL suggests crypto project".into());
    }

    if matches!(input.source, "cryptoskill" | "web3-mcp-hub") {
        score += 15;
        reasons.push(format!("discovered via crypto registry ({})", input.source));
    }

    let has_medium_crypto_signal = reasons.iter().any(|r| {
        r.contains("web3")
            || r.contains("blockchain")
            || r.contains("crypto")
            || r.contains("on-chain")
    });

    if has_evidence(input) {
        score += 5;
        reasons.push("has trustworthy listing evidence".into());
    } else {
        negative_signals.push("sparse evidence (no repo/homepage/npm/MCP endpoint)".into());
        if has_medium_crypto_signal {
            // Borderline tools with a crypto hint but no URLs stay in manual review range.
            score = score.max(42);
        } else {
            score = score.saturating_sub(10);
        }
    }

    for (keyword, label) in REJECT_CATEGORIES {
        if matches_keyword(&text, keyword) {
            negative_signals.push(label.to_string());
            score = score.saturating_sub(35);
        }
    }

    let weak_only = count_weak_only_signals(&text);
    let strong_or_medium = reasons.iter().any(|r| {
        !r.contains("weak alone")
            && !r.contains("MCP surface")
            && !r.contains("SDK surface")
            && !r.contains("agent surface")
            && !r.contains("gateway surface")
            && !r.contains("API surface")
            && !r.contains("trustworthy listing evidence")
    });

    if weak_only >= 1 && !strong_or_medium && input.chains.is_empty() {
        negative_signals.push("only generic MCP/SDK/agent/gateway/api keywords".into());
        score = score.min(25);
    }

    let keyword_hits = count_crypto_keyword_hits(&text);
    if keyword_hits >= 4 && !has_evidence(input) {
        negative_signals.push("keyword stuffing without listing evidence".into());
        score = score.min(55);
    }

    score = score.clamp(0, 100);

    let status = if !negative_signals.is_empty()
        && negative_signals
            .iter()
            .any(|s| s.contains("filesystem") || s.contains("weather") || s.contains("calendar"))
        && score < 50
    {
        "rejected".to_string()
    } else if score >= 70 {
        "accepted".to_string()
    } else if score >= 40 {
        "needs_review".to_string()
    } else {
        "rejected".to_string()
    };

    RelevanceAssessment {
        score,
        status,
        reasons,
        negative_signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(
        name: &'a str,
        description: Option<&'a str>,
        chains: &'a [String],
        repo: Option<&'a str>,
        npm: Option<&'a str>,
    ) -> RelevanceInput<'a> {
        RelevanceInput {
            name,
            description,
            tool_type: "mcp",
            repo_url: repo,
            homepage: None,
            npm_package: npm,
            mcp_endpoint: None,
            chains,
            source: "github",
        }
    }

    #[test]
    fn accepted_bob_gateway_cli() {
        let chains = vec!["bitcoin".into(), "ethereum".into(), "base".into()];
        let assessment = assess_relevance(&input(
            "BOB Gateway CLI",
            Some("Bitcoin to EVM bridge CLI for cross-chain transfers"),
            &chains,
            Some("https://github.com/bob-collective/bob"),
            Some("@gobob/gateway-cli"),
        ));
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
        let assessment = assess_relevance(&input(
            "Chain Wallet MCP",
            Some("MCP server for wallet operations and chain signing"),
            &chains,
            Some("https://github.com/example/wallet-mcp"),
            None,
        ));
        assert_eq!(assessment.status, "accepted");
        assert!(assessment.score >= 70);
        assert!(assessment.reasons.iter().any(|r| r.contains("wallet")));
    }

    #[test]
    fn needs_review_generic_web3_helper_sparse_evidence() {
        let assessment = assess_relevance(&input(
            "Web3 Helper",
            Some("A generic web3 helper utility"),
            &[],
            None,
            None,
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
        let assessment = assess_relevance(&input(
            "Filesystem MCP",
            Some("MCP server for local file operations"),
            &[],
            None,
            None,
        ));
        assert_eq!(assessment.status, "rejected");
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("filesystem")));
    }

    #[test]
    fn rejected_weather_calendar_mcp() {
        let weather = assess_relevance(&input(
            "Weather MCP",
            Some("MCP server for weather forecasts"),
            &[],
            None,
            None,
        ));
        assert_eq!(weather.status, "rejected");
        assert!(weather
            .negative_signals
            .iter()
            .any(|s| s.contains("weather")));

        let calendar = assess_relevance(&input(
            "Calendar MCP",
            Some("MCP integration for calendar events"),
            &[],
            None,
            None,
        ));
        assert_eq!(calendar.status, "rejected");
        assert!(calendar
            .negative_signals
            .iter()
            .any(|s| s.contains("calendar")));
    }

    #[test]
    fn keyword_stuffing_without_evidence_needs_review_or_rejected() {
        let assessment = assess_relevance(&input(
            "Crypto Mega Tool",
            Some(
                "web3 defi wallet bridge bitcoin ethereum solana crypto blockchain token nft staking dex onchain x402 rwa evm",
            ),
            &[],
            None,
            None,
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
        let assessment = assess_relevance(&input(
            "Generic MCP",
            Some("An MCP server"),
            &[],
            None,
            None,
        ));
        assert_ne!(assessment.status, "accepted");
        assert!(assessment
            .negative_signals
            .iter()
            .any(|s| s.contains("generic") || s.contains("sparse")));
    }

    #[test]
    fn cryptoskill_source_boosts_borderline_tool() {
        let chains = vec!["ethereum".into()];
        let mut inp = input(
            "Swap Router SDK",
            Some("SDK for DEX routing on EVM chains"),
            &chains,
            Some("https://github.com/example/swap-router"),
            Some("@example/swap-router"),
        );
        inp.source = "cryptoskill";
        let assessment = assess_relevance(&inp);
        assert!(assessment.score >= 40);
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.contains("crypto registry")));
    }
}
