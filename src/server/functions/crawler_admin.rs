use super::*;

/// Known crawler sources for the admin dashboard (merged with DB rows).
pub(crate) const CRAWLER_SOURCE_DEFS: &[(&str, &str)] = &[
    ("cryptoskill", "Every 6h"),
    ("github", "Hourly (+30m offset)"),
    ("npm", "Hourly"),
    ("web3-mcp-hub", "Every 12h"),
];

/// Admin crawler row — source status plus schedule hint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlerSourceView {
    pub name: String,
    pub url: String,
    pub schedule: String,
    pub last_crawled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub crawl_status: String,
    pub items_found: i32,
    pub error_message: Option<String>,
}

/// Build crawler source rows for admin views (shared by dashboard and crawler page).
#[cfg(feature = "ssr")]
pub(crate) async fn list_crawler_sources_inner(
    pool: &sqlx::PgPool,
) -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let rows = sqlx::query_as::<_, Source>("SELECT * FROM sources ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to list sources: {e}")))?;

    let mut by_name: std::collections::HashMap<String, Source> =
        rows.into_iter().map(|r| (r.name.clone(), r)).collect();

    let mut views = Vec::with_capacity(CRAWLER_SOURCE_DEFS.len() + 1);
    for (name, schedule) in CRAWLER_SOURCE_DEFS {
        let url = default_source_registry_url(name).to_string();
        if let Some(row) = by_name.remove(*name) {
            views.push(CrawlerSourceView {
                name: row.name,
                url: row.url,
                schedule: (*schedule).into(),
                last_crawled_at: row.last_crawled_at,
                crawl_status: row.crawl_status,
                items_found: row.items_found,
                error_message: row.error_message,
            });
        } else {
            views.push(CrawlerSourceView {
                name: (*name).into(),
                url,
                schedule: (*schedule).into(),
                last_crawled_at: None,
                crawl_status: "pending".into(),
                items_found: 0,
                error_message: None,
            });
        }
    }

    for (_, row) in by_name {
        views.push(CrawlerSourceView {
            name: row.name,
            url: row.url,
            schedule: "—".into(),
            last_crawled_at: row.last_crawled_at,
            crawl_status: row.crawl_status,
            items_found: row.items_found,
            error_message: row.error_message,
        });
    }

    Ok(views)
}

/// List crawler source status (admin).
#[server(ListCrawlerSources, "/api")]
pub async fn list_crawler_sources() -> Result<Vec<CrawlerSourceView>, ServerFnError> {
    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;
    list_crawler_sources_inner(&pool).await
}

/// Validate manual crawler trigger input.
pub(crate) fn validate_trigger_crawler_source(source: &str) -> Result<(), &'static str> {
    match source {
        "npm" | "cryptoskill" | "web3-mcp-hub" | "github" | "sync_stars" => Ok(()),
        _ => Err("unknown crawler source"),
    }
}

/// Manually trigger a crawler job in the background (admin).
#[server(TriggerCrawlerSource, "/api")]
pub async fn trigger_crawler_source(source: String) -> Result<(), ServerFnError> {
    if let Err(msg) = validate_trigger_crawler_source(&source) {
        return Err(ServerFnError::new(msg.to_string()));
    }

    let (parts, pool, config) = request_context()?;
    require_admin(&parts, &pool, &config).await?;

    let pool_bg = pool.clone();
    let source_bg = source.clone();
    tokio::spawn(async move {
        crawler::trigger_source(&pool_bg, &source_bg).await;
    });

    Ok(())
}
