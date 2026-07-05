//! Premium MCP tool implementations (compare_tools, export_toolkit).

use crate::discovery::normalize_compare_slugs;
use crate::models::tool::{sanitize_tool_for_public_response, PublicTool};
use crate::models::Tool;
use crate::server::functions::{build_toolkit_payload, ToolComparisonView, ToolkitToolView};
use crate::server::queries::{APPROVED_TOOLS_BY_SLUGS_SQL, PUBLIC_TOOL_WHERE};
use crate::server::review_persistence::list_public_official_links;
use crate::trust_verification::verify_tool_trust;
use sqlx::PgPool;

const MAX_EXPORT_SLUGS: usize = 25;

pub async fn mcp_compare_tools(pool: &PgPool, slugs_raw: &str) -> Result<String, String> {
    let normalized = normalize_compare_slugs(slugs_raw);
    if normalized.len() < 2 {
        return Err("compare_tools requires 2–4 unique slugs (comma-separated)".into());
    }

    let tools = sqlx::query_as::<_, Tool>(APPROVED_TOOLS_BY_SLUGS_SQL)
        .bind(&normalized)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("db error: {e}"))?;

    let tool_map: std::collections::HashMap<String, Tool> = tools
        .into_iter()
        .map(|tool| (tool.slug.clone(), sanitize_tool_for_public_response(tool)))
        .collect();

    let mut rows = Vec::new();
    for slug in &normalized {
        let Some(tool) = tool_map.get(slug) else {
            continue;
        };
        let official_links = list_public_official_links(pool, tool.id)
            .await
            .map_err(|e| format!("official links failed: {e}"))?;
        let trust = verify_tool_trust(tool, &official_links);
        rows.push(ToolComparisonView {
            tool: PublicTool::from(tool.clone()),
            official_links,
            trust_facts: trust.trust_facts,
            viewer_bookmarked: false,
        });
    }

    if rows.len() < 2 {
        return Err("compare_tools found fewer than 2 public tools for the given slugs".into());
    }

    serde_json::to_string(&rows).map_err(|e| format!("serialize error: {e}"))
}

pub async fn mcp_export_toolkit(
    pool: &PgPool,
    slugs: Option<Vec<String>>,
    category: Option<&str>,
) -> Result<String, String> {
    let slugs = if let Some(slugs) = slugs {
        normalize_export_slugs(slugs)?
    } else if let Some(category) = category.map(str::trim).filter(|v| !v.is_empty()) {
        fetch_category_slugs(pool, category).await?
    } else {
        return Err("export_toolkit requires slugs[] or category".into());
    };

    if slugs.is_empty() {
        return Err("export_toolkit found no tools to export".into());
    }

    let tools = sqlx::query_as::<_, Tool>(APPROVED_TOOLS_BY_SLUGS_SQL)
        .bind(&slugs)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("db error: {e}"))?;

    let items: Vec<ToolkitToolView> = tools
        .into_iter()
        .map(|tool| ToolkitToolView {
            tool: PublicTool::from(sanitize_tool_for_public_response(tool)),
            note: None,
            tags: Vec::new(),
            source: "mcp".into(),
            source_client: Some("mcp".into()),
            saved_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .collect();

    let payload = build_toolkit_payload(items).map_err(|e| e.to_string())?;
    serde_json::to_string(&payload).map_err(|e| format!("serialize error: {e}"))
}

fn normalize_export_slugs(slugs: Vec<String>) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    let normalized = slugs
        .into_iter()
        .map(|slug| slug.trim().to_ascii_lowercase())
        .filter(|slug| !slug.is_empty())
        .filter(|slug| seen.insert(slug.clone()))
        .take(MAX_EXPORT_SLUGS)
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return Err("export_toolkit slugs must include at least one slug".into());
    }
    Ok(normalized)
}

async fn fetch_category_slugs(pool: &PgPool, category: &str) -> Result<Vec<String>, String> {
    let category = category.to_ascii_lowercase();
    let rows = sqlx::query_scalar::<_, String>(&format!(
        r#"
        SELECT slug
        FROM tools
        WHERE {PUBLIC_TOOL_WHERE}
          AND function = $1
        ORDER BY trust_score DESC, stars DESC, updated_at DESC
        LIMIT $2
        "#
    ))
    .bind(category)
    .bind(MAX_EXPORT_SLUGS as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("category export failed: {e}"))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_export_slugs_dedupes_and_caps() {
        let slugs = vec!["A".into(), "a".into(), "b".into()];
        let out = normalize_export_slugs(slugs).expect("slugs");
        assert_eq!(out, vec!["a", "b"]);
    }
}
