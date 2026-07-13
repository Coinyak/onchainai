//! Blueprint node sync from agents.
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::sync_tool::sync_tool;
use super::types::*;
use crate::server::fn_error::FnError;

pub fn agent_session_title(now: DateTime<Utc>) -> String {
    format!("Agent session · {}", now.format("%Y-%m-%d"))
}

pub(crate) fn snap_to_grid(value: i32) -> i32 {
    ((value + BLUEPRINT_GRID / 2) / BLUEPRINT_GRID) * BLUEPRINT_GRID
}

pub(crate) fn next_agent_tool_node_coords(nodes: &Value) -> (i32, i32) {
    let Some(arr) = nodes.as_array() else {
        return (
            snap_to_grid(AGENT_SESSION_START_X),
            snap_to_grid(AGENT_SESSION_START_Y),
        );
    };

    let mut max_bottom = -1;
    let mut anchor_x = AGENT_SESSION_START_X;

    for item in arr {
        if item.get("kind").and_then(|v| v.as_str()) != Some("tool") {
            continue;
        }
        let x = item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let bottom = y + BLUEPRINT_NODE_TOOL_HEIGHT;
        if bottom > max_bottom {
            max_bottom = bottom;
            anchor_x = x;
        }
    }

    if max_bottom < 0 {
        return (
            snap_to_grid(AGENT_SESSION_START_X),
            snap_to_grid(AGENT_SESSION_START_Y),
        );
    }

    (
        snap_to_grid(anchor_x),
        snap_to_grid(max_bottom + AGENT_NODE_STACK_GAP),
    )
}

pub(crate) fn slug_on_canvas(nodes: &Value, slug: &str) -> bool {
    nodes.as_array().is_some_and(|arr| {
        arr.iter().any(|node| {
            node.get("kind").and_then(|v| v.as_str()) == Some("tool")
                && node
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s == slug)
        })
    })
}

fn normalize_agent_tool_chains(chains: Option<&[String]>) -> Result<Vec<String>, FnError> {
    let Some(values) = chains else {
        return Ok(Vec::new());
    };
    if values.len() > MAX_TOOL_NODE_CHAINS {
        return Err(FnError::new(format!(
            "at most {MAX_TOOL_NODE_CHAINS} chains per tool node"
        )));
    }
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in values {
        let chain_id = raw.trim().to_lowercase();
        if chain_id.is_empty() {
            continue;
        }
        if chain_id.chars().count() > MAX_CHAIN_ID_LEN {
            return Err(FnError::new(format!(
                "chain id must be at most {MAX_CHAIN_ID_LEN} characters"
            )));
        }
        if !chain_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(FnError::new("chain id contains invalid characters"));
        }
        if seen.insert(chain_id.clone()) {
            normalized.push(chain_id);
        }
    }
    Ok(normalized)
}

fn initial_tool_node_chains(tool_chains: &[String]) -> Vec<String> {
    if tool_chains.len() == 1 {
        let id = tool_chains[0].trim().to_lowercase();
        if !id.is_empty() {
            return vec![id];
        }
    }
    Vec::new()
}

async fn load_tool_chains(pool: &PgPool, slug: &str) -> Result<Vec<String>, FnError> {
    let chains: Option<Vec<String>> = sqlx::query_scalar(
        r#"
        SELECT chains
        FROM tools
        WHERE slug = $1
          AND approval_status = 'approved'
          AND relevance_status = 'accepted'
          AND quarantined_at IS NULL
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("tool chains lookup failed: {e}")))?;

    Ok(chains.unwrap_or_default())
}

async fn find_or_create_agent_blueprint(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
) -> Result<(Uuid, Value, Value), FnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        nodes: Value,
        edges: Value,
    }

    if let Some(row) = sqlx::query_as::<_, Row>(
        "SELECT id, nodes, edges FROM blueprints WHERE user_id = $1 AND title = $2 LIMIT 1",
    )
    .bind(user_id)
    .bind(title)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint lookup failed: {e}")))?
    {
        return Ok((row.id, row.nodes, row.edges));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blueprints WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint count failed: {e}")))?;

    if count >= MAX_BLUEPRINTS_PER_USER {
        return Err(FnError::new(format!(
            "you can save at most {MAX_BLUEPRINTS_PER_USER} blueprints"
        )));
    }

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO blueprints (user_id, title, nodes, edges)
        VALUES ($1, $2, '[]'::jsonb, '[]'::jsonb)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint create failed: {e}")))?;

    Ok((id, json!([]), json!([])))
}

async fn approved_tool_exists(pool: &PgPool, slug: &str) -> Result<bool, FnError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM tools
            WHERE slug = $1
              AND approval_status = 'approved'
              AND relevance_status = 'accepted'
              AND quarantined_at IS NULL
        )
        "#,
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("tool lookup failed: {e}")))
}

async fn load_blueprint_canvas(
    pool: &PgPool,
    user_id: Uuid,
    blueprint_id: Option<Uuid>,
    session_title: &str,
) -> Result<(Uuid, Value), FnError> {
    if let Some(id) = blueprint_id {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            nodes: Value,
            edges: Value,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, nodes, edges FROM blueprints WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| FnError::new(format!("blueprint load failed: {e}")))?
        .ok_or_else(|| FnError::new("blueprint not found"))?;
        return Ok((row.id, row.nodes));
    }
    let (id, nodes, _) = find_or_create_agent_blueprint(pool, user_id, session_title).await?;
    Ok((id, nodes))
}

async fn blueprint_updated_at(
    pool: &PgPool,
    blueprint_id: Uuid,
    user_id: Uuid,
) -> Result<DateTime<Utc>, FnError> {
    sqlx::query_scalar::<_, DateTime<Utc>>(
        "SELECT updated_at FROM blueprints WHERE id = $1 AND user_id = $2",
    )
    .bind(blueprint_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint timestamp failed: {e}")))
}

async fn skip_blueprint_sync(
    pool: &PgPool,
    auth: &AgentAuth,
    slug: String,
    blueprint_id: Uuid,
    skip_reason: &str,
    idempotency_key: Option<&str>,
) -> Result<SyncBlueprintNodeResponse, FnError> {
    let updated_at = blueprint_updated_at(pool, blueprint_id, auth.user_id).await?;
    let response = SyncBlueprintNodeResponse {
        ok: true,
        slug,
        blueprint_id,
        node_id: None,
        appended: false,
        skip_reason: Some(skip_reason.into()),
        updated_at,
    };
    if let Some(key) = idempotency_key.filter(|k| !k.is_empty()) {
        let _ = record_blueprint_sync_log(pool, auth, &response.slug, key, &response).await;
    }
    Ok(response)
}

fn resolve_node_chains(
    requested: Option<&[String]>,
    tool_chains: &[String],
) -> Result<Vec<String>, FnError> {
    if let Some(values) = requested {
        let normalized = normalize_agent_tool_chains(Some(values))?;
        if normalized.is_empty() {
            return Ok(initial_tool_node_chains(tool_chains));
        }
        return Ok(normalized);
    }
    Ok(initial_tool_node_chains(tool_chains))
}

pub async fn sync_blueprint_node(
    pool: &PgPool,
    auth: &AgentAuth,
    req: SyncBlueprintNodeRequest,
) -> Result<SyncBlueprintNodeResponse, FnError> {
    if !auth.scopes.iter().any(|s| s == "blueprint:write") {
        return Err(FnError::new("token missing blueprint:write scope"));
    }

    let slug = req.slug.trim().to_string();
    if slug.is_empty() {
        return Err(FnError::new("slug required"));
    }

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        if let Some(cached) = load_idempotent_blueprint_sync(pool, auth.user_id, key).await? {
            return Ok(cached);
        }
    }

    if !approved_tool_exists(pool, &slug).await? {
        return Err(FnError::new(format!("tool not found: {slug}")));
    }

    let session_title = agent_session_title(Utc::now());
    let (blueprint_id, mut nodes) =
        load_blueprint_canvas(pool, auth.user_id, req.blueprint_id, &session_title).await?;

    let idem = req.idempotency_key.as_deref();
    if slug_on_canvas(&nodes, &slug) {
        return skip_blueprint_sync(pool, auth, slug, blueprint_id, "duplicate_slug", idem).await;
    }

    let node_count = nodes.as_array().map(|a| a.len()).unwrap_or(0);
    if node_count >= BLUEPRINT_MAX_NODES {
        return skip_blueprint_sync(pool, auth, slug, blueprint_id, "node_limit", idem).await;
    }

    let tool_chains = load_tool_chains(pool, &slug).await?;
    let chains = resolve_node_chains(req.chains.as_deref(), &tool_chains)?;

    let (x, y) = next_agent_tool_node_coords(&nodes);
    let node_id = Uuid::new_v4().to_string();
    let mut node = json!({
        "id": node_id,
        "kind": "tool",
        "slug": slug,
        "x": x,
        "y": y,
    });
    if !chains.is_empty() {
        node["chains"] = json!(chains);
    }

    let mut arr = nodes.as_array().cloned().unwrap_or_default();
    arr.push(node);
    nodes = Value::Array(arr);

    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        UPDATE blueprints
        SET nodes = $3
        WHERE id = $1 AND user_id = $2
        RETURNING updated_at
        "#,
    )
    .bind(blueprint_id)
    .bind(auth.user_id)
    .bind(&nodes)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("blueprint append failed: {e}")))?;

    let response = SyncBlueprintNodeResponse {
        ok: true,
        slug: slug.clone(),
        blueprint_id,
        node_id: Some(node_id),
        appended: true,
        skip_reason: None,
        updated_at,
    };

    if let Some(key) = req.idempotency_key.as_deref().filter(|k| !k.is_empty()) {
        let _ = record_blueprint_sync_log(pool, auth, &slug, key, &response).await;
    }

    Ok(response)
}

pub async fn save_stack_to_blueprint(
    pool: &PgPool,
    auth: &AgentAuth,
    slugs: Vec<String>,
    title: Option<String>,
) -> Result<Value, FnError> {
    if slugs.is_empty() {
        return Err(FnError::new("slugs required"));
    }
    if slugs.len() > 25 {
        return Err(FnError::new("at most 25 slugs per stack save"));
    }

    let blueprint_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| agent_session_title(Utc::now()));

    let (blueprint_id, _, _) =
        find_or_create_agent_blueprint(pool, auth.user_id, &blueprint_title).await?;

    let mut toolkit_results = Vec::new();
    let mut blueprint_results = Vec::new();

    for slug in slugs {
        let slug = slug.trim().to_string();
        if slug.is_empty() {
            continue;
        }
        let toolkit = sync_tool(
            pool,
            auth,
            SyncToolRequest {
                slug: slug.clone(),
                note: None,
                tags: None,
                source_client: Some("mcp".into()),
                idempotency_key: None,
            },
        )
        .await?;
        toolkit_results.push(toolkit);

        let blueprint = sync_blueprint_node(
            pool,
            auth,
            SyncBlueprintNodeRequest {
                blueprint_id: Some(blueprint_id),
                slug,
                chains: None,
                idempotency_key: None,
            },
        )
        .await?;
        blueprint_results.push(blueprint);
    }

    Ok(json!({
        "ok": true,
        "blueprint_id": blueprint_id,
        "title": blueprint_title,
        "toolkit": toolkit_results,
        "blueprint": blueprint_results,
    }))
}

async fn load_idempotent_blueprint_sync(
    pool: &PgPool,
    user_id: Uuid,
    key: &str,
) -> Result<Option<SyncBlueprintNodeResponse>, FnError> {
    let detail: Option<Value> = sqlx::query_scalar(
        r#"
        SELECT detail
        FROM agent_sync_log
        WHERE user_id = $1 AND idempotency_key = $2 AND action = 'sync_blueprint_node' AND status = 'ok'
        "#,
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|e| FnError::new(format!("idempotency lookup failed: {e}")))?;

    let Some(detail) = detail else {
        return Ok(None);
    };
    serde_json::from_value(detail)
        .map(Some)
        .map_err(|e| FnError::new(format!("idempotency decode failed: {e}")))
}

async fn record_blueprint_sync_log(
    pool: &PgPool,
    auth: &AgentAuth,
    slug: &str,
    key: &str,
    response: &SyncBlueprintNodeResponse,
) -> Result<(), FnError> {
    let detail = serde_json::to_value(response)
        .map_err(|e| FnError::new(format!("sync log serialize failed: {e}")))?;
    let _ = sqlx::query(
        r#"
        INSERT INTO agent_sync_log (user_id, agent_token_id, action, tool_slug, blueprint_id, idempotency_key, detail)
        VALUES ($1, $2, 'sync_blueprint_node', $3, $4, $5, $6)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.token_id)
    .bind(slug)
    .bind(response.blueprint_id)
    .bind(key)
    .bind(detail)
    .execute(pool)
    .await;
    Ok(())
}
