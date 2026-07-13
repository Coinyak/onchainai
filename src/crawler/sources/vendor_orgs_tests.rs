//! Tests extracted from `vendor_orgs.rs` for Code Health scoring.

use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sample_repos_json() -> String {
    serde_json::json!([
        {
            "id": 1,
            "name": "skills",
            "full_name": "circlefin/skills",
            "description": "Agent skills",
            "html_url": "https://github.com/circlefin/skills",
            "fork": false,
            "archived": false,
            "stargazers_count": 50,
            "pushed_at": "2026-06-01T12:00:00Z",
            "topics": ["mcp-server"],
            "language": "TypeScript"
        },
        {
            "id": 2,
            "name": "forked-tool",
            "full_name": "circlefin/forked-tool",
            "description": "fork",
            "html_url": "https://github.com/circlefin/forked-tool",
            "fork": true,
            "archived": false,
            "stargazers_count": 100,
            "pushed_at": "2026-06-01T12:00:00Z",
            "topics": [],
            "language": "Rust"
        },
        {
            "id": 3,
            "name": "archived-lib",
            "full_name": "circlefin/archived-lib",
            "description": "old",
            "html_url": "https://github.com/circlefin/archived-lib",
            "fork": false,
            "archived": true,
            "stargazers_count": 100,
            "pushed_at": "2026-06-01T12:00:00Z",
            "topics": [],
            "language": "Rust"
        },
        {
            "id": 4,
            "name": "low-star",
            "full_name": "circlefin/low-star",
            "description": "unpopular",
            "html_url": "https://github.com/circlefin/low-star",
            "fork": false,
            "archived": false,
            "stargazers_count": 2,
            "pushed_at": "2026-06-01T12:00:00Z",
            "topics": [],
            "language": "Rust"
        },
        {
            "id": 5,
            "name": "stale-repo",
            "full_name": "circlefin/stale-repo",
            "description": "stale",
            "html_url": "https://github.com/circlefin/stale-repo",
            "fork": false,
            "archived": false,
            "stargazers_count": 20,
            "pushed_at": "2020-01-01T00:00:00Z",
            "topics": [],
            "language": "Rust"
        },
        {
            "id": 6,
            "name": "gateway-contracts",
            "full_name": "circlefin/gateway-contracts",
            "description": "EVM gateway",
            "html_url": "https://github.com/circlefin/gateway-contracts",
            "fork": false,
            "archived": false,
            "stargazers_count": 40,
            "pushed_at": "2026-05-15T08:00:00Z",
            "topics": ["api", "mcp-server"],
            "language": "Solidity"
        }
    ])
    .to_string()
}

#[tokio::test]
async fn vendor_orgs_crawl_without_pool_returns_error() {
    let crawler = VendorOrgsCrawler;
    let err = crawler
        .crawl()
        .await
        .expect_err("pool-less crawl must fail");
    assert!(err.to_string().contains("crawl_with_pool"));
}

#[test]
fn vendor_orgs_effective_tool_name_renames_short_and_generic_names() {
    assert!(should_rename_repo_slug("skills"));
    assert!(should_rename_repo_slug("api"));
    assert!(should_rename_repo_slug("cli"));
    assert!(should_rename_repo_slug("abcd"));
    assert!(!should_rename_repo_slug("gateway-contracts"));

    assert_eq!(
        effective_tool_name("circlefin", "skills"),
        "circlefin-skills"
    );
    assert_eq!(
        effective_tool_name("circlefin", "gateway-contracts"),
        "gateway-contracts"
    );
}

#[test]
fn vendor_orgs_filter_excludes_fork_archived_low_star_and_stale() {
    let repos: Vec<OrgRepo> = serde_json::from_str(&sample_repos_json()).unwrap();
    let now = DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let kept = filter_org_repos(&repos, now);
    let names: Vec<_> = kept.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"skills"));
    assert!(names.contains(&"gateway-contracts"));
    assert!(!names.contains(&"forked-tool"));
    assert!(!names.contains(&"archived-lib"));
    assert!(!names.contains(&"low-star"));
    assert!(!names.contains(&"stale-repo"));
}

#[test]
fn vendor_orgs_is_non_tool_repo_identifies_non_tools() {
    assert!(is_non_tool_repo("docs"));
    assert!(is_non_tool_repo("documentation"));
    assert!(is_non_tool_repo("documentation-en"));
    assert!(is_non_tool_repo("doc.linea"));
    assert!(is_non_tool_repo("consensus-specs"));
    assert!(is_non_tool_repo("execution-specs"));
    assert!(is_non_tool_repo("execution-apis"));
    assert!(is_non_tool_repo("devp2p"));
    assert!(is_non_tool_repo("program-examples"));
    assert!(is_non_tool_repo("demo-basic-connect"));
    assert!(is_non_tool_repo("onramp-v2-mobile-demo"));
    assert!(is_non_tool_repo("ansible-role-besu"));
    assert!(is_non_tool_repo("ethereum-helm-charts"));
    assert!(is_non_tool_repo("eth-phishing-detect"));
    assert!(is_non_tool_repo("wormhole-audits"));
    assert!(is_non_tool_repo("safe-apps-list"));
    assert!(is_non_tool_repo("safe-transaction-service"));
    assert!(is_non_tool_repo("sun-network"));
    assert!(is_non_tool_repo("x402.chat"));
    assert!(is_non_tool_repo("pay-skills"));
    assert!(is_non_tool_repo("contract-deployments"));
    assert!(is_non_tool_repo("account-policies"));
    assert!(is_non_tool_repo("action-is-release"));
    assert!(is_non_tool_repo("contributor-docs"));
    assert!(is_non_tool_repo("esp-website"));
    assert!(is_non_tool_repo("ethereum-org-website"));
    assert!(is_non_tool_repo("zkvm-website"));
    assert!(is_non_tool_repo(".github"));
    assert!(is_non_tool_repo("TEPs"));
    assert!(is_non_tool_repo("lz-address-book"));
}

#[test]
fn vendor_orgs_is_non_tool_repo_preserves_real_tools() {
    assert!(!is_non_tool_repo("aave-sdk"));
    assert!(!is_non_tool_repo("go-ethereum"));
    assert!(!is_non_tool_repo("metamask-extension"));
    assert!(!is_non_tool_repo("agentkit"));
    assert!(!is_non_tool_repo("onchainkit"));
    assert!(!is_non_tool_repo("wallet-cli"));
    assert!(!is_non_tool_repo("layerzero-v2"));
    assert!(!is_non_tool_repo("mesh-sdk-go"));
    assert!(!is_non_tool_repo("safe-cli-nodejs"));
    assert!(!is_non_tool_repo("snap-bitcoin-wallet"));
}

#[test]
fn vendor_orgs_filter_excludes_non_tool_repos() {
    let now = Utc::now();
    let repos: Vec<OrgRepo> = vec![
        OrgRepo {
            id: 1,
            name: "consensus-specs".into(),
            full_name: "ethereum/consensus-specs".into(),
            description: Some("Ethereum PoS consensus specifications".into()),
            html_url: "https://github.com/ethereum/consensus-specs".into(),
            fork: false,
            archived: false,
            stargazers_count: 3000,
            pushed_at: Some("2026-06-15T00:00:00Z".into()),
            topics: vec!["mcp-server".into()],
            language: Some("Python".into()),
        },
        OrgRepo {
            id: 2,
            name: "agent-sdk".into(),
            full_name: "WalletConnect/agent-sdk".into(),
            description: Some("WalletConnect agent SDK for AI agents".into()),
            html_url: "https://github.com/WalletConnect/agent-sdk".into(),
            fork: false,
            archived: false,
            stargazers_count: 500,
            pushed_at: Some("2026-06-20T00:00:00Z".into()),
            topics: vec!["agent".into()],
            language: Some("TypeScript".into()),
        },
    ];
    let kept = filter_org_repos(&repos, now);
    let names: Vec<_> = kept.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"agent-sdk"));
    assert!(!names.contains(&"consensus-specs"));
}

#[test]
fn vendor_orgs_map_excludes_existing_repo_urls() {
    let repos: Vec<OrgRepo> = serde_json::from_str(&sample_repos_json()).unwrap();
    let now = DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut existing = HashSet::new();
    existing.insert("https://github.com/circlefin/gateway-contracts".to_string());
    let raws = map_org_repos_to_raws("circlefin", "Circle", &repos, &existing, now);
    assert_eq!(raws.len(), 1);
    assert_eq!(raws[0].name, "circlefin-skills");
    assert_eq!(raws[0].official_team.as_deref(), Some("Circle"));
}

#[tokio::test]
async fn vendor_orgs_wiremock_renames_generic_repo_name() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/orgs/circlefin/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sample_repos_json()))
        .mount(&server)
        .await;

    let client = github_client(None).unwrap();
    let repos = fetch_org_repos_at_url(&client, None, "circlefin", &server.uri())
        .await
        .expect("org repos fetch");
    let now = DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let raws = map_org_repos_to_raws("circlefin", "Circle", &repos, &HashSet::new(), now);
    assert!(raws.iter().any(|r| r.name == "circlefin-skills"));
    assert!(raws.iter().all(|r| r.name != "skills"));
}

#[tokio::test]
async fn vendor_orgs_wiremock_excludes_fork_archived_and_low_star() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/orgs/circlefin/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sample_repos_json()))
        .mount(&server)
        .await;

    let client = github_client(None).unwrap();
    let repos = fetch_org_repos_at_url(&client, None, "circlefin", &server.uri())
        .await
        .expect("org repos fetch");
    let now = DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let kept = filter_org_repos(&repos, now);
    let names: HashSet<_> = kept.iter().map(|r| r.name.as_str()).collect();
    assert!(!names.contains("forked-tool"));
    assert!(!names.contains("archived-lib"));
    assert!(!names.contains("low-star"));
}

#[test]
fn vendor_orgs_filter_caps_per_org_at_max() {
    let now = Utc::now();
    let repos: Vec<OrgRepo> = (0..40)
        .map(|i| OrgRepo {
            id: i,
            name: format!("mcp-repo-{i}"),
            full_name: format!("vendor/mcp-repo-{i}"),
            description: Some("agent mcp server".into()),
            html_url: format!("https://github.com/vendor/mcp-repo-{i}"),
            fork: false,
            archived: false,
            stargazers_count: 10 + i as i32,
            pushed_at: Some(format!("2026-06-{:02}T00:00:00Z", (i % 28) + 1)),
            topics: vec!["mcp-server".into()],
            language: None,
        })
        .collect();
    assert_eq!(filter_org_repos(&repos, now).len(), MAX_REPOS_PER_ORG);
}

#[test]
fn vendor_orgs_agent_surface_gate_includes_x402_reference_repo() {
    let repo = OrgRepo {
        id: 1,
        name: "x402".into(),
        full_name: "x402-foundation/x402".into(),
        description: Some("A payments protocol for the internet. Built on HTTP.".into()),
        html_url: "https://github.com/x402-foundation/x402".into(),
        fork: false,
        archived: false,
        stargazers_count: 6255,
        pushed_at: Some("2026-07-06T09:17:39Z".into()),
        topics: vec![],
        language: Some("TypeScript".into()),
    };
    assert!(has_agent_surface(&repo));
}

#[test]
fn vendor_orgs_agent_surface_gate_excludes_plain_sdk_repos() {
    let repo = OrgRepo {
        id: 1,
        name: "v4-core".into(),
        full_name: "uniswap/v4-core".into(),
        description: Some("EVM contracts".into()),
        html_url: "https://github.com/uniswap/v4-core".into(),
        fork: false,
        archived: false,
        stargazers_count: 100,
        pushed_at: Some("2026-06-01T00:00:00Z".into()),
        topics: vec![],
        language: Some("Solidity".into()),
    };
    assert!(!has_agent_surface(&repo));
    let mcp_repo = OrgRepo {
        topics: vec!["mcp-server".into()],
        ..repo.clone()
    };
    assert!(has_agent_surface(&mcp_repo));
}
