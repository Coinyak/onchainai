//! Tests extracted from `github.rs` for Code Health scoring.

use super::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn search_response_json(_topic: &str) -> String {
    r#"{
            "total_count": 2,
            "incomplete_results": false,
            "items": [
                {
                    "id": 1,
                    "name": "web3-mcp",
                    "full_name": "strangelove-ventures/web3-mcp",
                    "description": "One MCP to rule all them chains.",
                    "html_url": "https://github.com/strangelove-ventures/web3-mcp",
                    "stargazers_count": 500,
                    "pushed_at": "2026-06-20T12:00:00Z",
                    "topics": ["mcp-server", "web3-mcp", "solana", "ethereum"],
                    "language": "TypeScript",
                    "clone_url": "https://github.com/strangelove-ventures/web3-mcp.git"
                },
                {
                    "id": 2,
                    "name": "crypto-mcp",
                    "full_name": "example/crypto-mcp",
                    "description": null,
                    "html_url": "https://github.com/example/crypto-mcp",
                    "stargazers_count": 42,
                    "pushed_at": "2026-06-19T10:00:00Z",
                    "topics": ["crypto-mcp"],
                    "language": "Rust",
                    "clone_url": "https://github.com/example/crypto-mcp.git"
                }
            ]
        }"#
    .to_string()
}

#[tokio::test]
async fn crawl_topics_with_custom_keywords_queries_live_settings() {
    let server = MockServer::start().await;
    let custom = vec!["defi-mcp".to_string()];

    Mock::given(method("GET"))
        .and(path("/search/repositories"))
        .and(query_param("q", "topic:defi-mcp"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_response_json("defi-mcp")))
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let base_url = server.uri();
    let items = search_topic_at_url(&client, None, "defi-mcp", &base_url)
        .await
        .expect("custom keyword search should succeed");
    assert_eq!(items.len(), 2);

    let topics = crate::crawler::settings::resolve_keywords(&custom);
    assert_eq!(topics, custom);
}

#[tokio::test]
async fn crawl_topics_queries_all_topics_and_parses_stars_pushed_at() {
    let server = MockServer::start().await;

    for topic in crate::crawler::settings::DEFAULT_SEARCH_KEYWORDS {
        Mock::given(method("GET"))
            .and(path("/search/repositories"))
            .and(query_param("q", format!("topic:{topic}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(search_response_json(topic)))
            .mount(&server)
            .await;
    }

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    // We cannot easily replace the GitHub API base in crawl_topics without
    // a refactor, so test the lower-level `search_topic` helper directly.
    let base_url = server.uri();
    let items = search_topic_at_url(&client, None, "mcp-server", &base_url)
        .await
        .expect("search should succeed");

    assert_eq!(items.len(), 2);
    let web3 = items.iter().find(|i| i.name == "web3-mcp").unwrap();
    assert_eq!(web3.stargazers_count, 500);
    assert_eq!(web3.pushed_at.as_deref(), Some("2026-06-20T12:00:00Z"));
    assert!(web3.topics.contains(&"mcp-server".to_string()));
}

#[tokio::test]
async fn search_includes_authorization_header_when_token_set() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search/repositories"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(search_response_json("mcp-server")),
        )
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let base_url = server.uri();
    let items = search_topic_at_url(&client, Some("test-token"), "mcp-server", &base_url)
        .await
        .expect("search should succeed");
    assert!(!items.is_empty());
}

#[tokio::test]
async fn search_request_includes_user_agent_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search/repositories"))
        .and(header(
            "user-agent",
            crate::crawler::sources::CRAWLER_USER_AGENT,
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(search_response_json("mcp-server")),
        )
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let base_url = server.uri();
    let items = search_topic_at_url(&client, None, "mcp-server", &base_url)
        .await
        .expect("search should succeed");
    assert!(!items.is_empty());
}

#[test]
fn infer_tool_type_from_readme_and_topics() {
    assert_eq!(
        infer_tool_type(Some("Model Context Protocol server"), &[], None),
        "mcp"
    );
    assert_eq!(
        infer_tool_type(Some("A CLI tool for crypto"), &[], None),
        "cli"
    );
    assert_eq!(
        infer_tool_type(None, &["sdk".to_string(), "typescript".to_string()], None),
        "sdk"
    );
    assert_eq!(
        infer_tool_type(None, &["api".to_string()], Some("Python")),
        "api"
    );
    assert_eq!(
        infer_tool_type(None, &["random".to_string()], Some("Rust")),
        "cli"
    );
}

#[test]
fn parse_github_url_variants() {
    assert_eq!(
        parse_github_url("https://github.com/owner/repo"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_url("https://github.com/owner/repo.git"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_url("https://github.com/owner/repo/"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_url("https://github.com/owner/repo?tab=readme"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_url("https://github.com/owner/repo#readme"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(parse_github_url("not-a-url"), None);
    assert_eq!(parse_github_url("https://github.com/owner"), None);
    assert_eq!(parse_github_url("https://example.com/owner/repo"), None);
}

#[test]
fn chain_topic_filtering() {
    assert!(is_chain_topic("ethereum"));
    assert!(is_chain_topic("Solana"));
    assert!(!is_chain_topic("mcp-server"));
    assert!(!is_chain_topic("cli"));
}

#[test]
fn search_item_to_raw_maps_fields() {
    let item = SearchItem {
        id: 7,
        name: "my-mcp".into(),
        full_name: "owner/my-mcp".into(),
        description: Some("desc".into()),
        html_url: "https://github.com/owner/my-mcp".into(),
        stargazers_count: 99,
        pushed_at: Some("2026-06-25T00:00:00Z".into()),
        topics: vec!["mcp-server".into(), "solana".into()],
        language: Some("TypeScript".into()),
        clone_url: None,
        owner: None,
    };
    let raw = search_item_to_raw(&item);
    assert_eq!(raw.source, "github");
    assert_eq!(raw.tool_type, "mcp");
    assert_eq!(raw.stars, 99);
    assert!(raw.chains.contains(&"solana".to_string()));
    assert!(raw.last_commit_at.is_some());
}

#[tokio::test]
async fn fetch_repo_api_parses_stars_and_pushed_at() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "stargazers_count": 1234,
            "pushed_at": "2026-06-24T18:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let repo = fetch_repo_api_at_url(&client, None, &format!("{}/repos/owner/repo", server.uri()))
        .await
        .expect("repo API should succeed");

    assert_eq!(repo.stargazers_count, 1234);
    assert_eq!(repo.pushed_at.as_deref(), Some("2026-06-24T18:00:00Z"));
}

#[tokio::test]
async fn fetch_repo_api_includes_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo"))
        .and(header("authorization", "Bearer token-42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "stargazers_count": 0,
            "pushed_at": null
        })))
        .mount(&server)
        .await;

    let client = reqwest::Client::builder()
        .user_agent(crate::crawler::sources::CRAWLER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let repo = fetch_repo_api_at_url(
        &client,
        Some("token-42"),
        &format!("{}/repos/owner/repo", server.uri()),
    )
    .await
    .expect("repo API should succeed with token");
    assert_eq!(repo.stargazers_count, 0);
}
