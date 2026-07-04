//! Tool model — maps the `tools` table.
//!
//! The DB column is named `type` (a Rust keyword), so the struct field is
//! `tool_type` with `#[sqlx(rename = "type")]` to map correctly.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A crypto tool row from the `tools` table.
///
/// See `migrations/001_init.sql` and `migrations/006_operator_hardening.sql` for
/// the full column list. All fields match the DB schema exactly; the `type`
/// column is renamed to `tool_type` because `type` is a reserved Rust keyword.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Tool {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,

    // Classification (3-axis + type)
    pub function: String,
    pub asset_class: String,
    pub actor: String,
    /// Maps to the DB column `type`. Renamed because `type` is a Rust keyword.
    #[cfg_attr(feature = "ssr", sqlx(rename = "type"))]
    #[serde(rename = "type")]
    pub tool_type: String,

    // Connections
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub install_command: Option<String>,
    pub mcp_endpoint: Option<String>,

    // Chain support
    pub chains: Vec<String>,

    // Trust
    pub status: String,
    pub official_team: Option<String>,
    pub trust_score: i32,

    // Approval (admin panel)
    pub approval_status: String,
    pub submitted_by: Option<Uuid>,
    pub rejection_reason: Option<String>,

    // Relevance and install safety (operator hardening)
    pub crypto_relevance_score: i32,
    pub crypto_relevance_reasons: Vec<String>,
    pub relevance_status: String,
    pub install_risk_level: String,
    pub install_risk_reasons: Vec<String>,
    pub requires_secret: bool,
    pub safe_copy_command: Option<String>,
    pub quarantined_at: Option<DateTime<Utc>>,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub review_policy_version: String,

    // Claim flow (operator hardening Phase 5)
    pub claim_state: String,

    // Meta
    pub license: Option<String>,
    pub pricing: String,
    pub x402_price: Option<String>,
    pub referral_enabled: bool,
    pub referral_bps: Option<i32>,
    pub referral_payout_address: Option<String>,
    pub referral_model: Option<String>,
    pub x402_pay_to_address: Option<String>,
    pub x402_builder_code: Option<String>,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
    pub x402_endpoint: Option<String>,
    pub x402_last_checked_at: Option<DateTime<Utc>>,
    pub x402_check_failures: i32,
    pub stars: i32,
    pub last_commit_at: Option<DateTime<Utc>>,

    // Source
    pub source: String,
    pub source_url: Option<String>,

    // Branding
    pub logo_url: Option<String>,
    pub logo_monogram: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Default operator-hardening field values for new or test tools.
#[derive(Debug, Clone)]
pub struct ToolReviewDefaults {
    pub crypto_relevance_score: i32,
    pub crypto_relevance_reasons: Vec<String>,
    pub relevance_status: String,
    pub install_risk_level: String,
    pub install_risk_reasons: Vec<String>,
    pub requires_secret: bool,
    pub safe_copy_command: Option<String>,
    pub quarantined_at: Option<DateTime<Utc>>,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub review_policy_version: String,
}

impl Default for ToolReviewDefaults {
    fn default() -> Self {
        Self {
            crypto_relevance_score: 0,
            crypto_relevance_reasons: vec![],
            relevance_status: "needs_review".into(),
            install_risk_level: "medium".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: "operator-hardening-v1".into(),
        }
    }
}

pub fn default_review_fields() -> ToolReviewDefaults {
    ToolReviewDefaults::default()
}

/// Parse a positive page number from a raw query value (`"2"`, `"abc"` -> `None`).
pub fn parse_page_value(raw: &str) -> Option<u32> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u32>().ok().filter(|page| *page > 0)
}

/// Whether `logo_url` is safe to render as an external image.
pub fn logo_url_is_safe_for_img(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.starts_with("/brand/")
        && trimmed.len() > "/brand/".len()
        && !trimmed.contains("..")
    {
        return true;
    }
    LogoUrlCandidate::parse(url).is_some_and(|candidate| candidate.is_renderable())
}

/// Back-compat alias for [`logo_url_is_safe_for_img`].
pub fn logo_url_is_http(url: &str) -> bool {
    logo_url_is_safe_for_img(url)
}

struct LogoUrlCandidate {
    parsed: url::Url,
    host: LogoHost,
}

impl LogoUrlCandidate {
    fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        LogoText(trimmed).is_safe().then_some(())?;
        let parsed = url::Url::parse(trimmed).ok()?;
        valid_logo_credentials(&parsed).then_some(())?;
        valid_logo_scheme(parsed.scheme()).then_some(())?;
        let host = LogoHost::new(parsed.host_str().filter(|host| !host.is_empty())?);
        Some(Self { parsed, host })
    }

    fn is_renderable(&self) -> bool {
        self.allowed_host() || self.safe_homepage_favicon()
    }

    fn allowed_host(&self) -> bool {
        self.host.github() || self.https_cdn_host()
    }

    fn https_cdn_host(&self) -> bool {
        self.parsed.scheme() == "https" && self.host.cdn()
    }

    fn safe_homepage_favicon(&self) -> bool {
        self.parsed.scheme() == "https" && self.parsed.path() == "/favicon.ico"
    }
}

struct LogoText<'a>(&'a str);

impl LogoText<'_> {
    fn is_safe(&self) -> bool {
        !self.has_forbidden_chars() && !self.has_blocked_scheme_prefix()
    }

    fn has_forbidden_chars(&self) -> bool {
        self.0.contains(['\"', '\'', '<', '>', '\r', '\n'])
    }

    fn has_blocked_scheme_prefix(&self) -> bool {
        let lower = self.0.to_ascii_lowercase();
        BLOCKED_LOGO_SCHEMES
            .iter()
            .any(|scheme| lower.starts_with(scheme))
    }
}

struct LogoHost(String);

impl LogoHost {
    fn new(host: &str) -> Self {
        Self(host.to_ascii_lowercase())
    }

    fn github(&self) -> bool {
        GITHUB_LOGO_HOSTS.contains(&self.0.as_str()) || self.0.ends_with(".githubusercontent.com")
    }

    fn cdn(&self) -> bool {
        HTTPS_CDN_LOGO_HOST_SUFFIXES
            .iter()
            .any(|suffix| self.0.ends_with(suffix))
            || HTTPS_CDN_LOGO_HOSTS.contains(&self.0.as_str())
    }
}

fn valid_logo_credentials(parsed: &url::Url) -> bool {
    parsed.username().is_empty() && parsed.password().is_none()
}

fn valid_logo_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
}

const BLOCKED_LOGO_SCHEMES: &[&str] = &["javascript:", "data:", "vbscript:", "file:", "blob:"];

const GITHUB_LOGO_HOSTS: &[&str] = &[
    "github.com",
    "avatars.githubusercontent.com",
    "raw.githubusercontent.com",
];

const HTTPS_CDN_LOGO_HOSTS: &[&str] = &["cdn.jsdelivr.net", "unpkg.com"];

const HTTPS_CDN_LOGO_HOST_SUFFIXES: &[&str] = &[
    ".cloudfront.net",
    ".amazonaws.com",
    ".jsdelivr.net",
    ".fastly.net",
];

/// Filter a stored logo URL through [`logo_url_is_safe_for_img`].
pub fn sanitize_logo_url(url: Option<String>) -> Option<String> {
    url.filter(|u| logo_url_is_safe_for_img(u))
}

struct GitHubOwner<'a>(&'a str);

impl GitHubOwner<'_> {
    fn is_valid(&self) -> bool {
        self.has_valid_length() && self.has_valid_hyphens() && self.has_valid_bytes()
    }

    fn has_valid_length(&self) -> bool {
        !self.0.is_empty() && self.0.len() <= 39
    }

    fn has_valid_hyphens(&self) -> bool {
        !self.0.starts_with('-') && !self.0.ends_with('-') && !self.0.contains("--")
    }

    fn has_valid_bytes(&self) -> bool {
        self.0
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }
}

/// Infer a GitHub owner avatar URL from a repository URL.
pub fn github_owner_avatar_url(repo_url: Option<&str>) -> Option<String> {
    let parsed = url::Url::parse(repo_url?.trim()).ok()?;
    if !parsed.host_str()?.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let mut segments = parsed.path_segments()?;
    let owner = segments.next()?.trim();
    let repo = segments.next()?.trim().trim_end_matches(".git");
    if !github_repo_path_is_valid(owner, repo) {
        return None;
    }
    Some(format!("https://avatars.githubusercontent.com/{owner}"))
}

fn github_repo_path_is_valid(owner: &str, repo: &str) -> bool {
    !owner.is_empty() && !repo.is_empty() && GitHubOwner(owner).is_valid()
}

fn safe_https_site_url(url: &str) -> Option<url::Url> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    parsed.host_str().filter(|host| !host.is_empty())?;
    Some(parsed)
}

/// Infer a first-party favicon URL from a tool homepage.
pub fn homepage_favicon_url(homepage: Option<&str>) -> Option<String> {
    let parsed = safe_https_site_url(homepage?)?;
    let host = parsed.host_str()?;
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Some(format!("https://{host}{port}/favicon.ico"))
}

/// First-party catalog tools ship a bundled brand asset (not GitHub avatars).
fn first_party_brand_logo(slug: &str) -> Option<String> {
    match slug {
        "onchainai" => Some("/brand/onchainai-logo.png".to_string()),
        _ => None,
    }
}

/// Logo URL to render for a tool, if safe.
pub fn tool_logo_img_url(tool: &Tool) -> Option<String> {
    first_party_brand_logo(&tool.slug)
        .or_else(|| sanitize_logo_url(tool.logo_url.clone()))
        .or_else(|| github_owner_avatar_url(tool.repo_url.as_deref()))
        .or_else(|| homepage_favicon_url(tool.homepage.as_deref()))
}

/// Strip unsafe `logo_url` and operator payout addresses before public API/MCP list/detail.
pub fn sanitize_tool_for_public_response(mut tool: Tool) -> Tool {
    tool.logo_url = tool_logo_img_url(&tool);
    tool.referral_payout_address = None;
    tool.x402_pay_to_address = None;
    tool.x402_endpoint = None;
    tool.x402_last_checked_at = None;
    tool.x402_check_failures = 0;
    tool
}

/// Sanitize a batch of tools for public API/MCP responses.
pub fn sanitize_tools_for_public_response(tools: Vec<Tool>) -> Vec<Tool> {
    tools
        .into_iter()
        .map(sanitize_tool_for_public_response)
        .collect()
}

/// Monogram from tool name: first two alphanumeric chars, uppercased.
pub fn monogram_from_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// Display monogram: DB override when set, else computed from name (max 4 chars).
pub fn display_monogram(tool: &Tool) -> String {
    let raw = tool
        .logo_monogram
        .as_deref()
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| monogram_from_name(&tool.name));
    raw.chars().take(4).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_page_value_cases(cases: &[(&str, Option<u32>)]) {
        for (raw, expected) in cases {
            assert_eq!(parse_page_value(raw), *expected);
        }
    }

    fn assert_logo_safety(cases: &[(&str, bool)]) {
        for (url, expected) in cases {
            assert_eq!(logo_url_is_safe_for_img(url), *expected, "url: {url}");
        }
    }

    fn assert_sanitize_logo_cases(cases: &[(Option<&str>, Option<&str>)]) {
        for (raw, expected) in cases {
            let sanitized = sanitize_logo_url(raw.map(str::to_string));
            assert_eq!(sanitized.as_deref(), *expected);
        }
    }

    fn assert_json_fields(json: &serde_json::Value, cases: &[(&str, serde_json::Value)]) {
        for (key, expected) in cases {
            assert_eq!(&json[*key], expected);
        }
    }

    fn sample_tool() -> Tool {
        let review = default_review_fields();
        Tool {
            id: Uuid::nil(),
            name: "Test".into(),
            slug: "test".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: None,
            homepage: None,
            npm_package: None,
            install_command: None,
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
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
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
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
            stars: 0,
            last_commit_at: None,
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    #[test]
    fn tool_serde_renames_type_field() {
        // The JSON key must be "type" so API/MCP responses match the DB column
        // name expected by clients and agents.
        let tool = sample_tool();
        let json = serde_json::to_string(&tool).expect("serialize tool");
        assert!(
            json.contains("\"type\":\"mcp\""),
            "serde should emit the `type` key, got: {json}"
        );
        assert!(
            !json.contains("tool_type"),
            "serde should NOT emit `tool_type`, got: {json}"
        );

        // Round-trip back into the struct.
        let back: Tool = serde_json::from_str(&json).expect("deserialize tool");
        assert_eq!(back.tool_type, "mcp");
    }

    #[test]
    fn monogram_from_name_takes_first_two_alphanumeric() {
        assert_eq!(monogram_from_name("BOB Gateway"), "BO");
        assert_eq!(monogram_from_name("  @foo/bar  "), "FO");
        assert_eq!(monogram_from_name("!!!"), "");
    }

    #[test]
    fn display_monogram_prefers_db_override() {
        let mut tool = sample_tool();
        tool.name = "Uniswap V4".into();
        assert_eq!(display_monogram(&tool), "UN");

        tool.logo_monogram = Some("UV".into());
        assert_eq!(display_monogram(&tool), "UV");

        tool.logo_monogram = Some("".into());
        assert_eq!(display_monogram(&tool), "UN");
    }

    #[test]
    fn parse_page_value_rejects_invalid_values() {
        assert_page_value_cases(&[
            ("2", Some(2)),
            ("01", Some(1)),
            ("abc", None),
            ("0", None),
            ("-1", None),
            ("", None),
            (" 2", None),
            ("2.5", None),
            ("4294967296", None),
        ]);
    }

    #[test]
    fn logo_url_is_safe_for_img_requires_allowlisted_hosts() {
        assert_logo_safety(&[
            ("https://avatars.githubusercontent.com/bob-collective", true),
            ("https://cdn.example.cloudfront.net/logo.png", true),
            ("https://cdn.jsdelivr.net/npm/pkg/logo.png", true),
            ("http://avatars.githubusercontent.com/u/1", true),
            ("http://raw.githubusercontent.com/org/repo/logo.png", true),
            ("http://github.com/org/repo", true),
            ("https://example.com/logo.png", false),
            ("https://tracker.evil.example/pixel.gif", false),
            ("http://example.com/logo.png", false),
            ("http://cdn.jsdelivr.net/pkg/logo.png", false),
            ("http://x.cloudfront.net/logo.png", false),
            ("javascript:alert(1)", false),
            ("data:image/png;base64,abc", false),
            ("vbscript:msgbox(1)", false),
            ("file:///etc/passwd", false),
            ("blob:https://example.com/uuid", false),
            ("//example.com/logo.png", false),
            ("/brand/onchainai-logo.png", true),
            ("/brand/../etc/passwd", false),
            ("/chains/ethereum.svg", false),
            ("https:///logo.png", false),
            ("https://user:pass@evil.example/logo", false),
            ("https://evil.example@other.example/logo", false),
            ("https://example.com/logo\"onerror=alert(1)", false),
        ]);
    }

    #[test]
    fn sanitize_logo_url_filters_unsafe_values() {
        assert_sanitize_logo_cases(&[
            (Some("javascript:alert(1)"), None),
            (
                Some("https://avatars.githubusercontent.com/acme"),
                Some("https://avatars.githubusercontent.com/acme"),
            ),
            (
                Some("https://gobob.xyz/favicon.ico"),
                Some("https://gobob.xyz/favicon.ico"),
            ),
            (Some("https://gobob.xyz/logo.png"), None),
        ]);
    }

    #[test]
    fn homepage_favicon_url_and_persisted_logo_match() {
        let favicon = homepage_favicon_url(Some("https://gobob.xyz/docs")).unwrap();
        assert_eq!(favicon, "https://gobob.xyz/favicon.ico");
        assert_eq!(
            sanitize_logo_url(Some(favicon)),
            Some("https://gobob.xyz/favicon.ico".into())
        );
    }

    #[test]
    fn sanitize_tool_for_public_response_strips_payout_addresses() {
        let mut tool = sample_tool();
        tool.referral_payout_address = Some("0xabc".into());
        tool.x402_pay_to_address = Some("0xdef".into());
        let sanitized = sanitize_tool_for_public_response(tool);
        assert_eq!(sanitized.referral_payout_address, None);
        assert_eq!(sanitized.x402_pay_to_address, None);
    }

    #[test]
    fn sanitize_tool_for_public_response_strips_x402_probe_fields() {
        let mut tool = sample_tool();
        tool.x402_endpoint = Some("https://pay.example.com/probe".into());
        tool.x402_check_failures = 3;
        let sanitized = sanitize_tool_for_public_response(tool);
        assert_eq!(sanitized.x402_endpoint, None);
        assert_eq!(sanitized.x402_check_failures, 0);
        assert!(sanitized.x402_last_checked_at.is_none());
    }

    #[test]
    fn sanitize_tool_for_public_response_strips_unsafe_logo() {
        let mut tool = sample_tool();
        tool.logo_url = Some("javascript:alert(1)".into());
        let sanitized = sanitize_tool_for_public_response(tool);
        assert_eq!(sanitized.logo_url, None);

        let mut tool = sample_tool();
        tool.logo_url = Some("https://avatars.githubusercontent.com/acme".into());
        let sanitized = sanitize_tool_for_public_response(tool);
        assert_eq!(
            sanitized.logo_url.as_deref(),
            Some("https://avatars.githubusercontent.com/acme")
        );
    }

    #[test]
    fn tool_logo_img_url_gates_render_path() {
        let mut tool = sample_tool();
        assert_eq!(tool_logo_img_url(&tool), None);

        tool.logo_url = Some("data:image/png;base64,abc".into());
        assert_eq!(tool_logo_img_url(&tool), None);

        tool.logo_url = Some("https://avatars.githubusercontent.com/acme".into());
        assert_eq!(
            tool_logo_img_url(&tool).as_deref(),
            Some("https://avatars.githubusercontent.com/acme")
        );
    }

    #[test]
    fn tool_logo_img_url_falls_back_to_repo_owner_avatar() {
        let mut tool = sample_tool();
        tool.repo_url = Some("https://github.com/bob-collective/bob.git".into());
        assert_eq!(
            tool_logo_img_url(&tool).as_deref(),
            Some("https://avatars.githubusercontent.com/bob-collective")
        );
    }

    #[test]
    fn tool_logo_img_url_prefers_first_party_brand_for_onchainai() {
        let mut tool = sample_tool();
        tool.slug = "onchainai".into();
        tool.logo_url = Some("https://avatars.githubusercontent.com/love".into());
        assert_eq!(
            tool_logo_img_url(&tool).as_deref(),
            Some("/brand/onchainai-logo.png")
        );
        let sanitized = sanitize_tool_for_public_response(tool);
        assert_eq!(
            sanitized.logo_url.as_deref(),
            Some("/brand/onchainai-logo.png")
        );
    }

    #[test]
    fn tool_logo_img_url_falls_back_to_homepage_favicon() {
        let mut tool = sample_tool();
        tool.homepage = Some("https://gobob.xyz/docs".into());
        assert_eq!(
            tool_logo_img_url(&tool).as_deref(),
            Some("https://gobob.xyz/favicon.ico")
        );

        tool.homepage = Some("http://gobob.xyz".into());
        assert_eq!(homepage_favicon_url(tool.homepage.as_deref()), None);
    }

    #[test]
    fn tool_serde_includes_review_fields() {
        let mut tool = sample_tool();
        tool.crypto_relevance_score = 72;
        tool.relevance_status = "accepted".into();
        tool.install_risk_level = "low".into();
        let json = serde_json::to_value(&tool).expect("serialize tool");
        assert_json_fields(
            &json,
            &[
                ("crypto_relevance_score", serde_json::json!(72)),
                ("relevance_status", serde_json::json!("accepted")),
                ("install_risk_level", serde_json::json!("low")),
                (
                    "review_policy_version",
                    serde_json::json!("operator-hardening-v1"),
                ),
            ],
        );
    }
}
