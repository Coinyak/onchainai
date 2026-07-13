//! Public tool listing, dashboard, and toolkit helpers.
use super::super::*;
use super::list::*;

/// Tools browser page size (must match `tools_browser::TOOL_PAGE_SIZE`).
pub const BROWSER_TOOL_PAGE_SIZE: u32 = 50;

/// Clamp browser `page` query param to the UI-visible window.
pub fn clamp_browser_page_param(page: u32) -> u32 {
    let max_page = (MAX_LIST_TOOLS_LIMIT as u32) / BROWSER_TOOL_PAGE_SIZE;
    page.max(1).min(max_page)
}

/// Cumulative list limit for browser pagination (`offset` always 0).
pub fn browser_visible_limit_for_page(page: u32) -> i64 {
    let page = clamp_browser_page_param(page);
    let limit = page
        .saturating_mul(BROWSER_TOOL_PAGE_SIZE)
        .min(MAX_LIST_TOOLS_LIMIT as u32);
    i64::from(limit)
}

/// Single RPC payload for `ToolsBrowser` — avoids client-side fan-out into 5–6 `/api` calls.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserDataPayload {
    pub categories: Vec<(Category, i64)>,
    pub chains: Vec<(String, i64)>,
    pub total: i64,
    pub tools: Vec<crate::models::tool::PublicToolSummary>,
    pub comment_counts: HashMap<String, i64>,
    pub preview_tool: Option<crate::models::tool::PublicTool>,
}

pub const MAX_DASHBOARD_LIST_LIMIT: i64 = 12;

pub fn clamp_dashboard_list_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_DASHBOARD_LIST_LIMIT)
}

fn encoded_query_value(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

pub fn dashboard_filter_href(axis: &str, value: &str) -> String {
    let value = encoded_query_value(value);
    match axis {
        "function" => format!("/tools?function={value}"),
        "type" => format!("/tools?type={value}"),
        "chain" => format!("/tools?chain={value}"),
        "status" => format!("/tools?status={value}"),
        "pricing" => format!("/tools?pricing={value}"),
        _ => "/tools".into(),
    }
}

fn dashboard_label(axis: &str, value: &str) -> String {
    match axis {
        "type" if value.eq_ignore_ascii_case("mcp") => "MCP".into(),
        "type" if value.eq_ignore_ascii_case("cli") => "CLI".into(),
        "type" if value.eq_ignore_ascii_case("sdk") => "SDK".into(),
        "type" if value.eq_ignore_ascii_case("api") => "API".into(),
        "type" if value.eq_ignore_ascii_case("x402") => "x402".into(),
        "status" if value.eq_ignore_ascii_case("official") => "Official".into(),
        "status" if value.eq_ignore_ascii_case("verified") => "Verified".into(),
        "status" if value.eq_ignore_ascii_case("community") => "Community".into(),
        "pricing" if value.eq_ignore_ascii_case("x402") => "x402".into(),
        _ => value
            .split(['-', '_'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn dashboard_bucket(axis: &str, id: String, label: Option<String>, count: i64) -> DashboardBucket {
    let label = label.unwrap_or_else(|| dashboard_label(axis, &id));
    let href = dashboard_filter_href(axis, &id);
    DashboardBucket {
        id,
        label,
        count,
        href,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DashboardBucket {
    pub id: String,
    pub label: String,
    pub count: i64,
    pub href: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardMetrics {
    pub public_tools: i64,
    pub mcp_tools: i64,
    pub cli_tools: i64,
    pub sdk_tools: i64,
    pub api_tools: i64,
    pub x402_tools: i64,
    pub official_tools: i64,
    pub verified_tools: i64,
    pub updated_recently: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublicDashboardSnapshot {
    pub metrics: DashboardMetrics,
    pub type_counts: Vec<DashboardBucket>,
    pub function_counts: Vec<DashboardBucket>,
    pub chain_counts: Vec<DashboardBucket>,
    pub trust_counts: Vec<DashboardBucket>,
    pub pricing_counts: Vec<DashboardBucket>,
    pub new_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub popular_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub x402_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub high_trust_tools: Vec<crate::models::tool::PublicToolSummary>,
    pub as_of: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolkitExportPayload {
    pub format: String,
    pub filename: String,
    pub body: String,
}

/// Public-safe tool shape for JSON export — omits internal ids and operator fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolkitExportTool {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub function: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub chains: Vec<String>,
    pub status: String,
    pub official_team: Option<String>,
    pub pricing: String,
    pub x402_price: Option<String>,
    pub install_command: Option<String>,
    pub safe_copy_command: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub license: Option<String>,
    pub stars: i32,
    pub claim_state: String,
    pub install_risk_level: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolkitToolView {
    pub tool: crate::models::tool::PublicTool,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub source: String,
    pub source_client: Option<String>,
    pub saved_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ToolkitToolView {
    pub fn from_tool(tool: Tool) -> Self {
        let now = chrono::Utc::now();
        Self {
            tool: crate::models::tool::PublicTool::from(tool),
            note: None,
            tags: Vec::new(),
            source: "web".into(),
            source_client: None,
            saved_at: now,
            updated_at: now,
        }
    }
}

fn tool_to_toolkit_export(item: &ToolkitToolView) -> ToolkitExportTool {
    let tool = &item.tool;
    ToolkitExportTool {
        slug: tool.slug.clone(),
        name: tool.name.clone(),
        description: tool.description.clone(),
        function: tool.function.clone(),
        tool_type: tool.tool_type.clone(),
        chains: tool.chains.clone(),
        status: tool.status.clone(),
        official_team: tool.official_team.clone(),
        pricing: tool.pricing.clone(),
        x402_price: tool.x402_price.clone(),
        install_command: tool.install_command.clone(),
        safe_copy_command: tool.safe_copy_command.clone(),
        mcp_endpoint: tool.mcp_endpoint.clone(),
        repo_url: tool.repo_url.clone(),
        homepage: tool.homepage.clone(),
        npm_package: tool.npm_package.clone(),
        license: tool.license.clone(),
        stars: tool.stars,
        claim_state: tool.claim_state.clone(),
        install_risk_level: tool.install_risk_level.clone(),
        note: item.note.clone(),
        tags: item.tags.clone(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MyToolkitPayload {
    pub total: i64,
    pub items: Vec<ToolkitToolView>,
    pub tools: Vec<crate::models::tool::PublicToolSummary>,
    pub markdown_export: ToolkitExportPayload,
    pub json_export: ToolkitExportPayload,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateToolkitItemPayload {
    pub slug: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolComparisonView {
    pub tool: crate::models::tool::PublicTool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust_facts: Vec<TrustFact>,
    pub viewer_bookmarked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_probe: Option<crate::server::trust_probe_meta::StaleTrustBadge>,
}

/// Request for bundled browser data load.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadBrowserDataRequest {
    pub sort: String,
    #[serde(default)]
    pub filters: ToolFilters,
    #[serde(default)]
    pub search_q: Option<String>,
    #[serde(default)]
    pub selected: Option<String>,
    pub page: u32,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
struct DashboardValueCountRow {
    id: String,
    count: i64,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
struct DashboardCategoryCountRow {
    id: String,
    label: String,
    count: i64,
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_value_counts(
    pool: &sqlx::PgPool,
    axis: DashboardCountAxis,
    limit: i64,
) -> Result<Vec<DashboardBucket>, FnError> {
    let rows = sqlx::query_as::<_, DashboardValueCountRow>(axis.count_sql())
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            FnError::new(format!(
                "{} dashboard counts failed: {e}",
                axis.bucket_axis()
            ))
        })?;

    Ok(rows
        .into_iter()
        .map(|row| dashboard_bucket(axis.bucket_axis(), row.id, None, row.count))
        .collect())
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_function_counts(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<DashboardBucket>, FnError> {
    let rows = sqlx::query_as::<_, DashboardCategoryCountRow>(DASHBOARD_FUNCTION_COUNTS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("function dashboard counts failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| dashboard_bucket("function", row.id, Some(row.label), row.count))
        .collect())
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_x402_tools(pool: &sqlx::PgPool, limit: i64) -> Result<Vec<Tool>, FnError> {
    let tools = sqlx::query_as::<_, Tool>(DASHBOARD_X402_TOOLS_SQL)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("x402 dashboard tools failed: {e}")))?;
    Ok(sanitize_tools_for_public_response(tools))
}

#[cfg(feature = "ssr")]
async fn fetch_dashboard_metrics(pool: &sqlx::PgPool) -> Result<DashboardMetrics, FnError> {
    let row =
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64)>(DASHBOARD_METRICS_SQL)
            .fetch_one(pool)
            .await
            .map_err(|e| FnError::new(format!("dashboard metrics failed: {e}")))?;

    Ok(DashboardMetrics {
        public_tools: row.0,
        mcp_tools: row.1,
        cli_tools: row.2,
        sdk_tools: row.3,
        api_tools: row.4,
        x402_tools: row.5,
        official_tools: row.6,
        verified_tools: row.7,
        updated_recently: row.8,
    })
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_public_dashboard_snapshot(
    pool: &sqlx::PgPool,
    list_limit: i64,
) -> Result<PublicDashboardSnapshot, FnError> {
    let limit = clamp_dashboard_list_limit(list_limit);
    let empty_filters = ToolFilters::default();
    let high_trust_filters = ToolFilters {
        status: vec!["official".into(), "verified".into()],
        ..Default::default()
    };

    let (metrics, type_counts, function_counts, chain_counts, trust_counts, pricing_counts) = futures::join!(
        fetch_dashboard_metrics(pool),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Type, limit),
        fetch_dashboard_function_counts(pool, limit),
        fetch_chain_counts(pool, limit),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Status, limit),
        fetch_dashboard_value_counts(pool, DashboardCountAxis::Pricing, limit),
    );
    let (new_tools, popular_tools, x402_tools, high_trust_tools) = futures::join!(
        fetch_list_tools(pool, "new", 0, limit, &empty_filters, None),
        fetch_list_tools(pool, "hot", 0, limit, &empty_filters, None),
        fetch_dashboard_x402_tools(pool, limit),
        fetch_list_tools(pool, "hot", 0, limit, &high_trust_filters, None),
    );

    use crate::models::tool::tools_to_public_summaries;

    Ok(PublicDashboardSnapshot {
        metrics: metrics?,
        type_counts: type_counts?,
        function_counts: function_counts?,
        chain_counts: chain_counts?
            .into_iter()
            .map(|(id, count)| dashboard_bucket("chain", id, None, count))
            .collect(),
        trust_counts: trust_counts?,
        pricing_counts: pricing_counts?,
        new_tools: tools_to_public_summaries(new_tools?),
        popular_tools: tools_to_public_summaries(popular_tools?),
        x402_tools: tools_to_public_summaries(x402_tools?),
        high_trust_tools: tools_to_public_summaries(high_trust_tools?),
        as_of: chrono::Utc::now(),
    })
}

fn sanitize_toolkit_items(items: Vec<ToolkitToolView>) -> Vec<ToolkitToolView> {
    items
        .into_iter()
        .map(|mut item| {
            let mut tool = item.tool;
            tool.name = redact_secrets(&tool.name);
            tool.description = tool.description.map(|value| redact_secrets(&value));
            tool.install_command = tool.install_command.map(|value| redact_secrets(&value));
            tool.safe_copy_command = tool.safe_copy_command.map(|value| redact_secrets(&value));
            tool.mcp_endpoint = tool.mcp_endpoint.map(|value| redact_secrets(&value));
            item.tool = tool;
            item.note = item.note.map(|value| redact_secrets(&value));
            item.tags = item
                .tags
                .into_iter()
                .map(|value| redact_secrets(&value))
                .collect();
            item
        })
        .collect()
}

fn toolkit_markdown_for_items(items: &[ToolkitToolView]) -> String {
    let mut body = String::from("# My OnchainAI Toolkit\n\n");
    if items.is_empty() {
        body.push_str("No saved tools yet.\n");
        return body;
    }

    body.push_str("Saved tools exported from OnchainAI.\n\n");
    for item in items {
        let tool = &item.tool;
        let chains = if tool.chains.is_empty() {
            "Not listed".into()
        } else {
            tool.chains.join(", ")
        };
        let install = tool
            .safe_copy_command
            .as_deref()
            .or(tool.install_command.as_deref())
            .unwrap_or("No install command listed");
        let endpoint = tool
            .mcp_endpoint
            .as_deref()
            .unwrap_or("No MCP endpoint listed");
        let _ = writeln!(body, "## {}", tool.name);
        let _ = writeln!(body, "- Slug: {}", tool.slug);
        let _ = writeln!(body, "- Type: {}", tool.tool_type);
        let _ = writeln!(body, "- Function: {}", tool.function);
        let _ = writeln!(body, "- Chains: {chains}");
        let _ = writeln!(body, "- Trust: {}", tool.status);
        let _ = writeln!(body, "- Pricing: {}", tool.pricing);
        if let Some(note) = item
            .note
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let _ = writeln!(body, "- Note: {note}");
        }
        if !item.tags.is_empty() {
            let _ = writeln!(body, "- Tags: {}", item.tags.join(", "));
        }
        if let Some(price) = tool.x402_price.as_deref().filter(|value| !value.is_empty()) {
            let _ = writeln!(body, "- x402 price: {price}");
        }
        let _ = writeln!(body, "- Install: `{install}`");
        let _ = writeln!(body, "- MCP endpoint: {endpoint}");
        let _ = writeln!(body, "- OnchainAI: /tools/{}\n", tool.slug);
    }
    body
}

pub fn build_toolkit_payload(items: Vec<ToolkitToolView>) -> Result<MyToolkitPayload, FnError> {
    use crate::models::tool::PublicToolSummary;
    let items = sanitize_toolkit_items(items);
    let tools: Vec<PublicToolSummary> = items
        .iter()
        .map(|item| PublicToolSummary::from(item.tool.clone()))
        .collect();
    let markdown_body = toolkit_markdown_for_items(&items);
    let export_tools: Vec<ToolkitExportTool> = items.iter().map(tool_to_toolkit_export).collect();
    let json_body = serde_json::to_string_pretty(&export_tools)
        .map_err(|e| FnError::new(format!("failed to serialize toolkit: {e}")))?;

    Ok(MyToolkitPayload {
        total: items.len() as i64,
        items,
        tools,
        markdown_export: ToolkitExportPayload {
            format: "markdown".into(),
            filename: "onchainai-toolkit.md".into(),
            body: markdown_body,
        },
        json_export: ToolkitExportPayload {
            format: "json".into(),
            filename: "onchainai-toolkit.json".into(),
            body: json_body,
        },
    })
}

#[cfg(feature = "ssr")]
async fn fetch_user_toolkit(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<MyToolkitPayload, FnError> {
    use sqlx::{FromRow, Row};

    let rows = sqlx::query(USER_TOOLKIT_SQL)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("failed to load toolkit: {e}")))?;

    let mut items = Vec::new();
    for row in rows {
        let tool = Tool::from_row(&row)
            .map_err(|e| FnError::new(format!("failed to decode toolkit tool: {e}")))?;
        let note = row
            .try_get::<Option<String>, _>("bookmark_note")
            .map_err(|e| FnError::new(format!("failed to decode toolkit note: {e}")))?;
        let tags = row
            .try_get::<Vec<String>, _>("bookmark_tags")
            .map_err(|e| FnError::new(format!("failed to decode toolkit tags: {e}")))?;
        let saved_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_created_at")
            .map_err(|e| FnError::new(format!("failed to decode toolkit saved_at: {e}")))?;
        let updated_at = row
            .try_get::<chrono::DateTime<chrono::Utc>, _>("bookmark_updated_at")
            .map_err(|e| FnError::new(format!("failed to decode toolkit updated_at: {e}")))?;
        let source = row
            .try_get::<String, _>("bookmark_source")
            .unwrap_or_else(|_| "web".into());
        let source_client = row
            .try_get::<Option<String>, _>("bookmark_source_client")
            .ok()
            .flatten();
        items.push(ToolkitToolView {
            tool: crate::models::tool::PublicTool::from(tool),
            note,
            tags,
            source,
            source_client,
            saved_at,
            updated_at,
        });
    }

    build_toolkit_payload(items)
}

pub(crate) fn validate_toolkit_tags(tags: &[String]) -> Result<Vec<String>, FnError> {
    if tags.len() > 8 {
        return Err(FnError::new("toolkit tags accept at most 8 values"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 32 {
            return Err(FnError::new("toolkit tags must be at most 32 characters"));
        }
        if tag
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
        {
            return Err(FnError::new(
                "toolkit tags may contain letters, numbers, hyphens, and underscores",
            ));
        }
        if seen.insert(tag.clone()) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn validate_toolkit_note(note: Option<String>) -> Result<Option<String>, FnError> {
    let note = note.map(|value| value.trim().to_string());
    let note = note.filter(|value| !value.is_empty());
    if note.as_ref().is_some_and(|value| value.len() > 500) {
        return Err(FnError::new("toolkit note must be at most 500 characters"));
    }
    Ok(note)
}

/// Batch comment counts for approved tools by slug.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_tool_comment_counts(
    pool: &sqlx::PgPool,
    slugs: &[String],
) -> Result<Vec<(String, i64)>, FnError> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, (String, i64)>(TOOL_COMMENT_COUNTS_BY_SLUGS_SQL)
        .bind(slugs)
        .fetch_all(pool)
        .await
        .map_err(|e| FnError::new(format!("comment counts failed: {e}")))?;

    Ok(rows)
}
