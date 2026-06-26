//! Hermes operator harness — bounded snapshots and read-only recommendations.
//!
//! Admin-gated Axum routes for agent-assisted operator review. Responses are
//! token-budgeted and secrets are redacted before leaving the server.

use crate::auth::guard::{require_admin, AuthError};
use crate::models::Tool;
use crate::server::functions::{derive_claim_state, derive_lifecycle_state, review_queue_where};
use crate::server::secret_redaction::redact_secrets;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: &str = "operator-snapshot-v1";
pub const MAX_TOOLS: usize = 25;
pub const MAX_DUPLICATES: usize = 3;
pub const MAX_EVIDENCE_SNIPPETS: usize = 5;
pub const MAX_SNIPPET_CHARS: usize = 500;
pub const MAX_DESCRIPTION_CHARS: usize = 300;
pub const MAX_INSTALL_COMMAND_CHARS: usize = 500;

/// Hermes may propose these action types (human approval still required).
pub const HERMES_ALLOWED_ACTIONS: &[&str] =
    &["approve", "reject", "needs_info", "quarantine", "outreach"];

/// Hermes must never directly execute or auto-propose these without human gate.
pub const HERMES_FORBIDDEN_ACTIONS: &[&str] = &[
    "deploy",
    "cleanup",
    "mark_official",
    "mark_verified",
    "auth_change",
    "rls_change",
    "delete_listing",
    "public_approval",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotLimits {
    pub max_tools: usize,
    pub max_duplicates_per_tool: usize,
    pub max_evidence_snippets: usize,
    pub max_snippet_chars: usize,
    pub max_description_chars: usize,
    pub max_install_command_chars: usize,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_tools: MAX_TOOLS,
            max_duplicates_per_tool: MAX_DUPLICATES,
            max_evidence_snippets: MAX_EVIDENCE_SNIPPETS,
            max_snippet_chars: MAX_SNIPPET_CHARS,
            max_description_chars: MAX_DESCRIPTION_CHARS,
            max_install_command_chars: MAX_INSTALL_COMMAND_CHARS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueSummary {
    pub queue: String,
    pub total_matching: i64,
    pub returned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateCandidateV1 {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceSnippetV1 {
    pub source: String,
    pub url: Option<String>,
    pub content_hash: Option<String>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorToolSnapshotV1 {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub source: String,
    pub source_url: Option<String>,
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub description_excerpt: Option<String>,
    pub install_command_excerpt: Option<String>,
    pub crypto_relevance_score: i32,
    pub relevance_status: String,
    pub relevance_reasons: Vec<String>,
    pub install_risk_level: String,
    pub install_risk_reasons: Vec<String>,
    pub duplicate_candidates: Vec<DuplicateCandidateV1>,
    pub evidence_snippets: Vec<EvidenceSnippetV1>,
    pub lifecycle_state: String,
    pub claim_state: String,
    pub stars: i32,
    pub last_commit_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorSnapshotV1 {
    pub snapshot_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
    pub limits: SnapshotLimits,
    pub truncated: bool,
    pub queue_summary: QueueSummary,
    pub tools: Vec<OperatorToolSnapshotV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendationEvidence {
    pub source: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorRecommendationV1 {
    pub tool_id: Uuid,
    pub tool_slug: String,
    pub action_type: String,
    pub confidence: f32,
    pub required_human_approval: bool,
    pub rationale: String,
    pub evidence: Vec<RecommendationEvidence>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub queue: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct OperatorRunRequest {
    pub snapshot_id: Option<Uuid>,
    pub queue: Option<String>,
    pub tool_ids: Option<Vec<Uuid>>,
    pub agent_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OperatorRunResponse {
    pub snapshot_id: Option<Uuid>,
    pub agent_name: String,
    pub recommendations: Vec<OperatorRecommendationV1>,
    pub read_only: bool,
}

/// Normalize harness queue aliases to operator review queue ids.
pub fn normalize_harness_queue(queue: &str) -> Result<&'static str, &'static str> {
    match queue {
        "pending" | "new_candidate" => Ok("new_candidate"),
        "known_update" => Ok("known_update"),
        "needs_manual_research" => Ok("needs_manual_research"),
        "low_relevance" => Ok("low_relevance"),
        "reported" => Ok("reported"),
        "high_risk_install" => Ok("high_risk_install"),
        _ => Err("unknown review queue"),
    }
}

/// Bound arbitrary text to a max char count (UTF-8 safe).
pub fn bound_text(input: Option<&str>, max_chars: usize) -> Option<String> {
    let text = input?.trim();
    if text.is_empty() {
        return None;
    }
    let redacted = redact_secrets(text);
    if redacted.chars().count() <= max_chars {
        Some(redacted)
    } else {
        let truncated: String = redacted.chars().take(max_chars).collect();
        Some(format!("{truncated}…"))
    }
}

/// Simple stable hash for evidence dedup (not cryptographic).
pub fn excerpt_hash(excerpt: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    excerpt.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Build bounded evidence snippets from tool fields (untrusted README-like content).
pub fn build_evidence_snippets(tool: &Tool, limits: &SnapshotLimits) -> Vec<EvidenceSnippetV1> {
    let mut snippets = Vec::new();

    if let Some(desc) = bound_text(tool.description.as_deref(), limits.max_snippet_chars) {
        snippets.push(EvidenceSnippetV1 {
            source: "description".into(),
            url: tool.source_url.clone(),
            content_hash: Some(excerpt_hash(&desc)),
            excerpt: desc,
        });
    }

    if let Some(cmd) = bound_text(tool.install_command.as_deref(), limits.max_snippet_chars) {
        snippets.push(EvidenceSnippetV1 {
            source: "install_command".into(),
            url: tool.repo_url.clone().or_else(|| tool.homepage.clone()),
            content_hash: Some(excerpt_hash(&cmd)),
            excerpt: cmd,
        });
    }

    for reason in tool
        .crypto_relevance_reasons
        .iter()
        .chain(tool.install_risk_reasons.iter())
    {
        if snippets.len() >= limits.max_evidence_snippets {
            break;
        }
        if let Some(excerpt) = bound_text(Some(reason.as_str()), limits.max_snippet_chars) {
            snippets.push(EvidenceSnippetV1 {
                source: "scanner_reason".into(),
                url: tool.source_url.clone(),
                content_hash: Some(excerpt_hash(&excerpt)),
                excerpt,
            });
        }
    }

    snippets.truncate(limits.max_evidence_snippets);
    snippets
}

/// Convert a DB tool row into a bounded harness snapshot entry.
pub fn tool_to_snapshot(
    tool: &Tool,
    duplicates: Vec<DuplicateCandidateV1>,
    limits: &SnapshotLimits,
) -> OperatorToolSnapshotV1 {
    let mut duplicate_candidates = duplicates;
    duplicate_candidates.truncate(limits.max_duplicates_per_tool);

    OperatorToolSnapshotV1 {
        id: tool.id,
        name: redact_secrets(&tool.name),
        slug: tool.slug.clone(),
        source: tool.source.clone(),
        source_url: tool
            .source_url
            .as_ref()
            .map(|u| redact_secrets(u))
            .filter(|u| !u.is_empty()),
        repo_url: tool
            .repo_url
            .as_ref()
            .map(|u| redact_secrets(u))
            .filter(|u| !u.is_empty()),
        homepage: tool
            .homepage
            .as_ref()
            .map(|u| redact_secrets(u))
            .filter(|u| !u.is_empty()),
        npm_package: tool.npm_package.clone(),
        description_excerpt: bound_text(tool.description.as_deref(), limits.max_description_chars),
        install_command_excerpt: bound_text(
            tool.install_command.as_deref(),
            limits.max_install_command_chars,
        ),
        crypto_relevance_score: tool.crypto_relevance_score,
        relevance_status: tool.relevance_status.clone(),
        relevance_reasons: tool
            .crypto_relevance_reasons
            .iter()
            .map(|r| redact_secrets(r))
            .take(5)
            .collect(),
        install_risk_level: tool.install_risk_level.clone(),
        install_risk_reasons: tool
            .install_risk_reasons
            .iter()
            .map(|r| redact_secrets(r))
            .take(5)
            .collect(),
        duplicate_candidates,
        evidence_snippets: build_evidence_snippets(tool, limits),
        lifecycle_state: derive_lifecycle_state(tool),
        claim_state: derive_claim_state(tool),
        stars: tool.stars,
        last_commit_at: tool.last_commit_at,
    }
}

/// Validate that an action type stays within the Hermes proposal boundary.
pub fn validate_hermes_action(action_type: &str) -> Result<(), &'static str> {
    if HERMES_FORBIDDEN_ACTIONS.contains(&action_type) {
        return Err("action forbidden for Hermes harness");
    }
    if HERMES_ALLOWED_ACTIONS.contains(&action_type) {
        Ok(())
    } else {
        Err("unknown Hermes action type")
    }
}

/// Read-only recommendation engine for a single tool snapshot.
pub fn recommend_for_tool(tool: &OperatorToolSnapshotV1) -> OperatorRecommendationV1 {
    let mut evidence = Vec::new();
    let (action_type, confidence, rationale): (&str, f32, String) = if tool.relevance_status
        == "rejected"
    {
        evidence.push(RecommendationEvidence {
            source: "relevance".into(),
            detail: format!(
                "relevance_status={} score={}",
                tool.relevance_status, tool.crypto_relevance_score
            ),
        });
        (
            "reject",
            0.92_f32,
            "Relevance scanner rejected this listing as not crypto-related.".to_string(),
        )
    } else if tool.install_risk_level == "critical" {
        evidence.push(RecommendationEvidence {
            source: "install_safety".into(),
            detail: format!("install_risk_level={}", tool.install_risk_level),
        });
        (
            "quarantine",
            0.9_f32,
            "Critical install risk — quarantine until a human operator reviews install command."
                .to_string(),
        )
    } else if tool.install_risk_level == "high" {
        evidence.push(RecommendationEvidence {
            source: "install_safety".into(),
            detail: format!("install_risk_level={}", tool.install_risk_level),
        });
        (
            "needs_info",
            0.78_f32,
            "High install risk — request safer install evidence before approval.".to_string(),
        )
    } else if tool.relevance_status == "needs_review" || tool.crypto_relevance_score < 50 {
        evidence.push(RecommendationEvidence {
            source: "relevance".into(),
            detail: format!(
                "relevance_status={} score={}",
                tool.relevance_status, tool.crypto_relevance_score
            ),
        });
        (
            "needs_info",
            0.7_f32,
            "Borderline crypto relevance — gather README or package metadata before deciding."
                .to_string(),
        )
    } else if !tool_has_trustworthy_url_from_snapshot(tool) {
        (
            "needs_info",
            0.75_f32,
            "Missing trustworthy repo, homepage, npm, or MCP endpoint.".to_string(),
        )
    } else if tool.claim_state == "unclaimed"
        && tool.stars >= 50
        && tool.crypto_relevance_score >= 70
    {
        evidence.push(RecommendationEvidence {
            source: "claim".into(),
            detail: format!("stars={} claim_state={}", tool.stars, tool.claim_state),
        });
        (
            "outreach",
            0.65_f32,
            "High-value unclaimed listing — draft outreach to verify maintainership.".to_string(),
        )
    } else {
        evidence.push(RecommendationEvidence {
            source: "relevance".into(),
            detail: format!(
                "relevance_status={} score={}",
                tool.relevance_status, tool.crypto_relevance_score
            ),
        });
        evidence.push(RecommendationEvidence {
            source: "install_safety".into(),
            detail: format!("install_risk_level={}", tool.install_risk_level),
        });
        (
            "approve",
            0.82_f32,
            "Accepted relevance with manageable install risk and trustworthy URLs.".to_string(),
        )
    };

    debug_assert!(validate_hermes_action(action_type).is_ok());

    OperatorRecommendationV1 {
        tool_id: tool.id,
        tool_slug: tool.slug.clone(),
        action_type: action_type.into(),
        confidence,
        required_human_approval: true,
        rationale: redact_secrets(&rationale),
        evidence: evidence
            .into_iter()
            .map(|e| RecommendationEvidence {
                source: e.source,
                detail: redact_secrets(&e.detail),
            })
            .collect(),
    }
}

fn tool_has_trustworthy_url_from_snapshot(tool: &OperatorToolSnapshotV1) -> bool {
    let valid = |value: &Option<String>| {
        value.as_ref().is_some_and(|u| {
            let trimmed = u.trim();
            trimmed.starts_with("https://") || trimmed.starts_with("http://")
        })
    };
    valid(&tool.repo_url)
        || valid(&tool.homepage)
        || tool
            .npm_package
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
}

/// Cap requested limit to harness hard max.
pub fn clamp_tool_limit(limit: Option<u32>) -> usize {
    limit
        .map(|n| n as usize)
        .unwrap_or(MAX_TOOLS)
        .clamp(1, MAX_TOOLS)
}

fn admin_denied_response() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}

async fn require_harness_admin(
    state: &AppState,
    req: Request<axum::body::Body>,
) -> Result<(), Response> {
    let (parts, _) = req.into_parts();
    match require_admin(
        &parts,
        &state.pool,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(AuthError::Unauthorized) | Err(AuthError::Forbidden) => Err(admin_denied_response()),
    }
}

#[cfg(feature = "ssr")]
async fn fetch_duplicate_candidates(
    pool: &sqlx::PgPool,
    tool: &Tool,
) -> Result<Vec<DuplicateCandidateV1>, String> {
    let repo = tool.repo_url.as_deref().unwrap_or("");
    let rows = if repo.is_empty() {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND lower(name) = lower($2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(tool.id)
        .bind(&tool.name)
        .bind(MAX_DUPLICATES as i64)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT slug, name
            FROM tools
            WHERE id <> $1
              AND approval_status = 'pending'
              AND repo_url = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(tool.id)
        .bind(repo)
        .bind(MAX_DUPLICATES as i64)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| format!("duplicate lookup failed: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|(slug, name)| DuplicateCandidateV1 { slug, name })
        .collect())
}

#[cfg(feature = "ssr")]
pub async fn build_operator_snapshot(
    pool: &sqlx::PgPool,
    queue: &str,
    limit: usize,
) -> Result<OperatorSnapshotV1, String> {
    let normalized = normalize_harness_queue(queue).map_err(|e| e.to_string())?;
    let where_clause = review_queue_where(normalized).map_err(|e| e.to_string())?;

    let count_sql = format!("SELECT COUNT(*)::bigint FROM tools WHERE {where_clause}");
    let total_matching: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("count failed: {e}"))?;

    let sql = format!("SELECT * FROM tools WHERE {where_clause} ORDER BY updated_at DESC LIMIT $1");
    let tools = sqlx::query_as::<_, Tool>(&sql)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("snapshot query failed: {e}"))?;

    let limits = SnapshotLimits::default();
    let truncated = total_matching > tools.len() as i64;
    let mut snapshots = Vec::with_capacity(tools.len());

    for tool in &tools {
        let duplicates = fetch_duplicate_candidates(pool, tool).await?;
        snapshots.push(tool_to_snapshot(tool, duplicates, &limits));
    }

    Ok(OperatorSnapshotV1 {
        snapshot_id: Uuid::new_v4(),
        created_at: Utc::now(),
        schema_version: SCHEMA_VERSION.into(),
        limits,
        truncated,
        queue_summary: QueueSummary {
            queue: normalized.into(),
            total_matching,
            returned: snapshots.len(),
        },
        tools: snapshots,
    })
}

/// `GET /api/admin/operator/snapshot?queue=pending&limit=25`
pub async fn get_operator_snapshot(
    State(state): State<AppState>,
    Query(query): Query<SnapshotQuery>,
    req: Request<axum::body::Body>,
) -> Response {
    if require_harness_admin(&state, req).await.is_err() {
        return admin_denied_response();
    }

    let queue = query.queue.as_deref().unwrap_or("pending");
    let limit = clamp_tool_limit(query.limit);

    match build_operator_snapshot(&state.pool, queue, limit).await {
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

/// `POST /api/admin/operator/run` — read-only recommendations, never mutates tools.
pub async fn post_operator_run(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> Response {
    let (admin_parts, body) = req.into_parts();
    if require_admin(
        &admin_parts,
        &state.pool,
        &state.config.jwt_secret,
        &state.config.jwt_issuer(),
    )
    .await
    .is_err()
    {
        return admin_denied_response();
    }

    let body_bytes = match axum::body::to_bytes(body, 1024 * 64).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid body").into_response(),
    };
    let run_req: OperatorRunRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("invalid json: {e}")).into_response(),
    };

    let agent_name = run_req
        .agent_name
        .as_deref()
        .unwrap_or("hermes")
        .to_string();

    let tools_to_evaluate: Vec<OperatorToolSnapshotV1> =
        if let Some(ids) = run_req.tool_ids.as_ref().filter(|ids| !ids.is_empty()) {
            let limits = SnapshotLimits::default();
            let mut snapshots = Vec::new();
            for id in ids.iter().take(MAX_TOOLS) {
                let tool = match sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&state.pool)
                    .await
                {
                    Ok(Some(t)) => t,
                    Ok(None) => continue,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}"))
                            .into_response();
                    }
                };
                let duplicates = match fetch_duplicate_candidates(&state.pool, &tool).await {
                    Ok(d) => d,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
                };
                snapshots.push(tool_to_snapshot(&tool, duplicates, &limits));
            }
            snapshots
        } else {
            let queue = run_req.queue.as_deref().unwrap_or("pending");
            let limit = clamp_tool_limit(None);
            match build_operator_snapshot(&state.pool, queue, limit).await {
                Ok(snapshot) => snapshot.tools,
                Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
            }
        };

    let recommendations: Vec<OperatorRecommendationV1> = tools_to_evaluate
        .iter()
        .map(recommend_for_tool)
        .filter(|rec| validate_hermes_action(&rec.action_type).is_ok())
        .collect();

    let response = OperatorRunResponse {
        snapshot_id: run_req.snapshot_id,
        agent_name,
        recommendations,
        read_only: true,
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;
    use crate::server::secret_redaction::assert_json_has_no_secrets;
    use chrono::Utc;

    fn sample_tool() -> Tool {
        let review = default_review_fields();
        Tool {
            id: Uuid::new_v4(),
            name: "Bridge MCP".into(),
            slug: "bridge-mcp".into(),
            description: Some("Ethereum bridge MCP for crypto agents.".into()),
            function: "bridge".into(),
            asset_class: "multi".into(),
            actor: "agent".into(),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/example/bridge".into()),
            homepage: None,
            npm_package: Some("@example/bridge".into()),
            install_command: Some("npm i @example/bridge".into()),
            mcp_endpoint: None,
            chains: vec!["ethereum".into()],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "pending".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 82,
            crypto_relevance_reasons: vec!["repo topic crypto-mcp".into()],
            relevance_status: "accepted".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: None,
            pricing: "free".into(),
            x402_price: None,
            stars: 120,
            last_commit_at: None,
            source: "github".into(),
            source_url: Some("https://github.com/example/bridge".into()),
            logo_url: None,
            logo_monogram: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn normalize_harness_queue_maps_pending_alias() {
        assert_eq!(normalize_harness_queue("pending").unwrap(), "new_candidate");
        assert_eq!(
            normalize_harness_queue("new_candidate").unwrap(),
            "new_candidate"
        );
        assert!(normalize_harness_queue("bogus").is_err());
    }

    #[test]
    fn clamp_tool_limit_enforces_hard_max() {
        assert_eq!(clamp_tool_limit(None), 25);
        assert_eq!(clamp_tool_limit(Some(100)), 25);
        assert_eq!(clamp_tool_limit(Some(0)), 1);
    }

    #[test]
    fn redact_secrets_masks_env_names_and_tokens() {
        let input = "JWT_SECRET=super-secret-key GITHUB_CLIENT_SECRET=abc SUPABASE_SERVICE_KEY=xyz token ghp_abcdefghijklmnopqrstuvwxyz";
        let out = redact_secrets(input);
        assert!(!out.contains("super-secret-key"));
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwxyz"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn bound_text_truncates_long_readme_excerpt() {
        let long = "a".repeat(600);
        let bounded = bound_text(Some(&long), 500).expect("bounded");
        assert!(bounded.chars().count() <= 501);
        assert!(bounded.ends_with('…'));
    }

    #[test]
    fn build_evidence_snippets_respects_max_count() {
        let mut tool = sample_tool();
        tool.crypto_relevance_reasons = (0..10).map(|i| format!("reason-{i}")).collect();
        let limits = SnapshotLimits::default();
        let snippets = build_evidence_snippets(&tool, &limits);
        assert!(snippets.len() <= MAX_EVIDENCE_SNIPPETS);
        for snippet in &snippets {
            assert!(snippet.excerpt.chars().count() <= MAX_SNIPPET_CHARS + 1);
        }
    }

    #[test]
    fn tool_to_snapshot_bounds_duplicates_and_redacts() {
        let tool = sample_tool();
        let duplicates: Vec<DuplicateCandidateV1> = (0..5)
            .map(|i| DuplicateCandidateV1 {
                slug: format!("dup-{i}"),
                name: format!("Dup {i}"),
            })
            .collect();
        let limits = SnapshotLimits::default();
        let snap = tool_to_snapshot(&tool, duplicates, &limits);
        assert!(snap.duplicate_candidates.len() <= MAX_DUPLICATES);
        assert!(snap.evidence_snippets.len() <= MAX_EVIDENCE_SNIPPETS);
        assert!(
            snap.description_excerpt.as_ref().unwrap().chars().count() <= MAX_DESCRIPTION_CHARS + 1
        );
    }

    #[test]
    fn validate_hermes_action_rejects_forbidden_ops() {
        assert!(validate_hermes_action("approve").is_ok());
        assert!(validate_hermes_action("outreach").is_ok());
        assert!(validate_hermes_action("deploy").is_err());
        assert!(validate_hermes_action("mark_official").is_err());
        assert!(validate_hermes_action("auth_change").is_err());
    }

    #[test]
    fn recommend_for_tool_suggests_reject_on_low_relevance() {
        let mut tool = sample_tool();
        tool.relevance_status = "rejected".into();
        let snap = tool_to_snapshot(&tool, vec![], &SnapshotLimits::default());
        let rec = recommend_for_tool(&snap);
        assert_eq!(rec.action_type, "reject");
        assert!(rec.required_human_approval);
        assert!(validate_hermes_action(&rec.action_type).is_ok());
    }

    #[test]
    fn recommend_for_tool_suggests_quarantine_on_critical_install() {
        let mut tool = sample_tool();
        tool.install_risk_level = "critical".into();
        let snap = tool_to_snapshot(&tool, vec![], &SnapshotLimits::default());
        let rec = recommend_for_tool(&snap);
        assert_eq!(rec.action_type, "quarantine");
        assert!(rec.required_human_approval);
    }

    #[test]
    fn recommend_for_tool_suggests_outreach_for_high_value_unclaimed() {
        let tool = sample_tool();
        let snap = tool_to_snapshot(&tool, vec![], &SnapshotLimits::default());
        let rec = recommend_for_tool(&snap);
        assert_eq!(rec.action_type, "outreach");
    }

    #[test]
    fn recommend_for_tool_suggests_approve_for_clean_candidate() {
        let mut tool = sample_tool();
        tool.stars = 5;
        tool.claim_state = "claimed".into();
        let snap = tool_to_snapshot(&tool, vec![], &SnapshotLimits::default());
        let rec = recommend_for_tool(&snap);
        assert_eq!(rec.action_type, "approve");
        assert!(rec.confidence > 0.5);
    }

    #[test]
    fn snapshot_limits_defaults_match_harness_constants() {
        let limits = SnapshotLimits::default();
        assert_eq!(limits.max_tools, 25);
        assert_eq!(limits.max_duplicates_per_tool, 3);
        assert_eq!(limits.max_evidence_snippets, 5);
        assert_eq!(limits.max_snippet_chars, 500);
    }

    #[test]
    fn redact_secrets_on_snapshot_json_roundtrip() {
        let mut tool = sample_tool();
        tool.description =
            Some("Uses JWT_SECRET=leaked-value and SUPABASE_SERVICE_KEY=also-leaked".into());
        let snap = tool_to_snapshot(&tool, vec![], &SnapshotLimits::default());
        let json = serde_json::to_string(&snap).expect("serialize");
        assert_json_has_no_secrets(&json);
    }

    #[test]
    fn evidence_snippets_include_provenance_and_bounds() {
        let tool = sample_tool();
        let limits = SnapshotLimits::default();
        let snippets = build_evidence_snippets(&tool, &limits);
        assert!(!snippets.is_empty());
        for snippet in snippets {
            assert!(!snippet.source.is_empty());
            assert!(snippet.content_hash.is_some());
            assert!(snippet.excerpt.chars().count() <= MAX_SNIPPET_CHARS + 1);
        }
    }
}
