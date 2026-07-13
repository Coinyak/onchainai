//! Crawler DB upsert, source status, and persist helpers.

use super::deduper;
use super::models;
use super::normalizer;
use super::settings;

/// PostgreSQL execution target for crawler upsert/persist helpers.
pub enum UpsertTarget<'a> {
    Pool(&'a sqlx::PgPool),
    /// Open transaction connection (`&mut *tx` from [`sqlx::PgPool::begin`]).
    Connection(&'a mut sqlx::PgConnection),
}

/// Upsert one crawled tool row (used by [`upsert_tools`] and DB integration tests).

pub fn gated_approval_status(
    source_name: &str,
    require_tool_approval: bool,
) -> &'static str {
    if source_name == "vendor_orgs" || source_name == "bazaar" {
        "pending"
    } else {
        settings::initial_approval_status(require_tool_approval)
    }
}

pub(crate) async fn upsert_one_tool<'e, E>(executor: E, tool: &models::Tool) -> anyhow::Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    use anyhow::Context;

    let logo_url = crate::models::tool::sanitize_logo_url(tool.logo_url.clone());
    sqlx::query(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, mcp_endpoint,
                chains, status, official_team, trust_score, approval_status,
                submitted_by, rejection_reason,
                crypto_relevance_score, crypto_relevance_reasons, relevance_status,
                install_risk_level, install_risk_reasons, requires_secret, safe_copy_command,
                review_policy_version, last_reviewed_at,
                license, pricing, x402_price, x402_endpoint,
                stars, last_commit_at, source, source_url, logo_url, logo_monogram,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                    $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36,
                    $37, $38, $39, now())
            ON CONFLICT (slug) DO UPDATE SET
                name = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN tools.name
                    ELSE EXCLUDED.name
                END,
                description = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.description, ''), EXCLUDED.description)
                    ELSE EXCLUDED.description
                END,
                function = EXCLUDED.function,
                asset_class = EXCLUDED.asset_class,
                actor = EXCLUDED.actor,
                type = EXCLUDED.type,
                repo_url = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.repo_url, ''), EXCLUDED.repo_url)
                    ELSE EXCLUDED.repo_url
                END,
                homepage = CASE
                    WHEN tools.status IN ('official', 'verified')
                         OR tools.claim_state = 'claimed'
                    THEN COALESCE(NULLIF(tools.homepage, ''), EXCLUDED.homepage)
                    ELSE EXCLUDED.homepage
                END,
                npm_package = EXCLUDED.npm_package,
                install_command = EXCLUDED.install_command,
                mcp_endpoint = EXCLUDED.mcp_endpoint,
                chains = EXCLUDED.chains,
                status = CASE
                    WHEN tools.status IN ('official', 'verified') THEN tools.status
                    ELSE EXCLUDED.status
                END,
                -- Never rehydrate official_team from crawl when the row is not
                -- official (verify-tool-official demotion clears the label; COALESCE
                -- with EXCLUDED would undo that on the next vendor_orgs pass).
                official_team = CASE
                    WHEN tools.status = 'official'
                    THEN COALESCE(tools.official_team, EXCLUDED.official_team)
                    ELSE tools.official_team
                END,
                trust_score = EXCLUDED.trust_score,
                approval_status = COALESCE(NULLIF(tools.approval_status, ''), EXCLUDED.approval_status),
                submitted_by = tools.submitted_by,
                rejection_reason = tools.rejection_reason,
                crypto_relevance_score = EXCLUDED.crypto_relevance_score,
                crypto_relevance_reasons = EXCLUDED.crypto_relevance_reasons,
                relevance_status = CASE
                    WHEN tools.last_reviewed_at IS NOT NULL THEN tools.relevance_status
                    ELSE EXCLUDED.relevance_status
                END,
                install_risk_level = EXCLUDED.install_risk_level,
                install_risk_reasons = EXCLUDED.install_risk_reasons,
                requires_secret = EXCLUDED.requires_secret,
                safe_copy_command = EXCLUDED.safe_copy_command,
                review_policy_version = EXCLUDED.review_policy_version,
                license = EXCLUDED.license,
                pricing = CASE
                    WHEN tools.pricing IN ('x402', 'paid', 'freemium') THEN tools.pricing
                    ELSE EXCLUDED.pricing
                END,
                x402_price = COALESCE(tools.x402_price, EXCLUDED.x402_price),
                x402_endpoint = COALESCE(tools.x402_endpoint, EXCLUDED.x402_endpoint),
                stars = EXCLUDED.stars,
                last_commit_at = EXCLUDED.last_commit_at,
                source = EXCLUDED.source,
                source_url = EXCLUDED.source_url,
                logo_url = EXCLUDED.logo_url,
                logo_monogram = COALESCE(EXCLUDED.logo_monogram, tools.logo_monogram),
                updated_at = now()
            "#,
        )
        .bind(&tool.name)
        .bind(&tool.slug)
        .bind(&tool.description)
        .bind(&tool.function)
        .bind(&tool.asset_class)
        .bind(&tool.actor)
        .bind(&tool.tool_type)
        .bind(&tool.repo_url)
        .bind(&tool.homepage)
        .bind(&tool.npm_package)
        .bind(&tool.install_command)
        .bind(&tool.mcp_endpoint)
        .bind(&tool.chains)
        .bind(&tool.status)
        .bind(&tool.official_team)
        .bind(tool.trust_score)
        .bind(&tool.approval_status)
        .bind(tool.submitted_by)
        .bind(&tool.rejection_reason)
        .bind(tool.crypto_relevance_score)
        .bind(&tool.crypto_relevance_reasons)
        .bind(&tool.relevance_status)
        .bind(&tool.install_risk_level)
        .bind(&tool.install_risk_reasons)
        .bind(tool.requires_secret)
        .bind(&tool.safe_copy_command)
        .bind(&tool.review_policy_version)
        .bind(tool.last_reviewed_at)
        .bind(&tool.license)
        .bind(&tool.pricing)
        .bind(&tool.x402_price)
        .bind(&tool.x402_endpoint)
        .bind(tool.stars)
        .bind(tool.last_commit_at)
        .bind(&tool.source)
        .bind(&tool.source_url)
        .bind(&logo_url)
        .bind(&tool.logo_monogram)
        .bind(tool.created_at)
        .execute(executor)
        .await
        .with_context(|| format!("upserting tool slug={}", tool.slug))?;
    Ok(())
}

/// Upsert a batch of crawled tools into the `tools` table.
///
/// Matching is by `slug` (unique). Existing rows keep their `status` and
/// `approval_status` when present (`official` / `verified` are preserved); all
/// other fields are overwritten with the freshly crawled values. This satisfies
/// VAL-CRAWL-014 (re-crawl preserves official/verified status).
pub async fn upsert_tools(target: UpsertTarget<'_>, tools: &[models::Tool]) -> anyhow::Result<()> {
    match target {
        UpsertTarget::Pool(pool) => {
            for tool in tools {
                upsert_one_tool(pool, tool).await?;
            }
        }
        UpsertTarget::Connection(conn) => {
            for tool in tools {
                upsert_one_tool(&mut *conn, tool).await?;
            }
        }
    }
    Ok(())
}

/// Registry URL written to `sources.url` for each built-in crawler name.
pub fn default_source_registry_url(source_name: &str) -> &'static str {
    match source_name {
        "npm" => "https://registry.npmjs.org/",
        "clawhub" => "https://clawhub.ai/api/v1",
        "cryptoskill" => "https://cryptoskill.org/skills.json",
        "web3-mcp-hub" => {
            "https://raw.githubusercontent.com/rudazy/web3-mcp-hub/main/registry.json"
        }
        "github" => "https://github.com/topics",
        "mcp-registry" => "https://registry.modelcontextprotocol.io/v0/servers",
        "vendor_orgs" => "https://api.github.com/orgs",
        "bazaar" => "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources",
        "pypi" => "https://pypi.org/",
        _ => "https://www.onchain-ai.xyz",
    }
}

/// Count raw crawl rows per `RawTool.source` (for diagnostics / status reporting).
pub fn count_raws_per_source(
    raws: &[normalizer::RawTool],
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for raw in raws {
        *counts.entry(raw.source.clone()).or_insert(0) += 1;
    }
    counts
}

pub async fn update_source_status(
    target: UpsertTarget<'_>,
    name: &str,
    url: &str,
    status: &str,
    items_found: i32,
    error_message: Option<&str>,
) {
    let result = match target {
        UpsertTarget::Pool(pool) => {
            update_source_status_one(pool, name, url, status, items_found, error_message).await
        }
        UpsertTarget::Connection(conn) => {
            update_source_status_one(&mut *conn, name, url, status, items_found, error_message)
                .await
        }
    };
    if let Err(e) = result {
        tracing::error!(source = name, error = %e, "failed to update sources table");
    }
}

pub(crate) async fn update_source_status_one<'e, E>(
    executor: E,
    name: &str,
    url: &str,
    status: &str,
    items_found: i32,
    error_message: Option<&str>,
) -> sqlx::Result<sqlx::postgres::PgQueryResult>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query(
        r#"
        INSERT INTO sources (name, url, last_crawled_at, crawl_status, items_found, error_message)
        VALUES ($1, $2, now(), $3, $4, $5)
        ON CONFLICT (name) DO UPDATE SET
            url = EXCLUDED.url,
            last_crawled_at = EXCLUDED.last_crawled_at,
            crawl_status = EXCLUDED.crawl_status,
            items_found = EXCLUDED.items_found,
            error_message = EXCLUDED.error_message,
            updated_at = now()
        "#,
    )
    .bind(name)
    .bind(url)
    .bind(status)
    .bind(items_found)
    .bind(error_message)
    .execute(executor)
    .await
}

/// DB-free pipeline: normalize raw crawls with gated approval, then dedupe.
pub fn prepare_crawled_tools_gated(
    raws: &[normalizer::RawTool],
    source_name: &str,
    require_tool_approval: bool,
) -> Vec<models::Tool> {
    let approval = gated_approval_status(source_name, require_tool_approval);
    let tools = normalizer::normalize_batch_with_status(raws, approval);
    deduper::dedupe(tools)
}

/// DB-free pipeline: normalize raw crawls with the approval decision, then dedupe.
///
/// Called by [`persist_crawl_results`] after loading `require_tool_approval` from
/// `site_settings`. Unit-tested without a database.
pub fn prepare_crawled_tools(
    raws: &[normalizer::RawTool],
    require_tool_approval: bool,
) -> Vec<models::Tool> {
    let approval = settings::initial_approval_status(require_tool_approval);
    let tools = normalizer::normalize_batch_with_status(raws, approval);
    deduper::dedupe(tools)
}

/// Normalize, dedupe, upsert crawled tools, then update the `sources` table.
///
/// Loads [`settings::CrawlerSettings`] to decide initial `approval_status` for
/// newly discovered tools (`pending` vs `approved`).
pub async fn persist_crawl_results(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
) {
    let crawler_settings = settings::load_crawler_settings(pool).await;
    let tools = prepare_crawled_tools(&raws, crawler_settings.require_tool_approval);
    let count = tools.len() as i32;
    match upsert_tools(UpsertTarget::Pool(pool), &tools).await {
        Ok(()) => {
            tracing::info!(source = name, count, "crawled tools upserted");
            update_source_status(UpsertTarget::Pool(pool), name, url, "success", count, None).await;
        }
        Err(e) => {
            tracing::error!(source = name, error = %e, "failed to upsert crawled tools");
            update_source_status(
                UpsertTarget::Pool(pool),
                name,
                url,
                "error",
                0,
                Some(&e.to_string()),
            )
            .await;
        }
    }
}

/// Persist gated crawl results with an explicit `require_tool_approval` flag.
///
/// Used by integration tests to prove vendor_orgs/bazaar force `pending` without
/// mutating `site_settings`. Production callers use [`persist_crawl_results_gated`].
pub async fn persist_crawl_results_gated_with_require(
    target: UpsertTarget<'_>,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
    require_tool_approval: bool,
) {
    let tools = prepare_crawled_tools_gated(&raws, name, require_tool_approval);
    let count = tools.len() as i32;

    match target {
        UpsertTarget::Pool(pool) => match upsert_tools(UpsertTarget::Pool(pool), &tools).await {
            Ok(()) => {
                tracing::info!(source = name, count, "crawled tools upserted (gated)");
                update_source_status(UpsertTarget::Pool(pool), name, url, "success", count, None)
                    .await;
            }
            Err(e) => {
                tracing::error!(source = name, error = %e, "failed to upsert gated crawled tools");
                update_source_status(
                    UpsertTarget::Pool(pool),
                    name,
                    url,
                    "error",
                    0,
                    Some(&e.to_string()),
                )
                .await;
            }
        },
        UpsertTarget::Connection(conn) => {
            match upsert_tools(UpsertTarget::Connection(conn), &tools).await {
                Ok(()) => {
                    tracing::info!(source = name, count, "crawled tools upserted (gated)");
                    update_source_status(
                        UpsertTarget::Connection(conn),
                        name,
                        url,
                        "success",
                        count,
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!(source = name, error = %e, "failed to upsert gated crawled tools");
                    update_source_status(
                        UpsertTarget::Connection(conn),
                        name,
                        url,
                        "error",
                        0,
                        Some(&e.to_string()),
                    )
                    .await;
                }
            }
        }
    }
}

/// Persist crawl results with per-source approval gating (§4.2).
///
/// For `vendor_orgs` and `bazaar`, newly normalized tools always get
/// `approval_status = "pending"`, regardless of `site_settings.require_tool_approval`.
pub async fn persist_crawl_results_gated(
    pool: &sqlx::PgPool,
    name: &str,
    url: &str,
    raws: Vec<normalizer::RawTool>,
) {
    let crawler_settings = settings::load_crawler_settings(pool).await;
    persist_crawl_results_gated_with_require(
        UpsertTarget::Pool(pool),
        name,
        url,
        raws,
        crawler_settings.require_tool_approval,
    )
    .await;
}
