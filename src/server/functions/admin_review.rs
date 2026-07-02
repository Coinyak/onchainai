use super::*;

/// SQL for admin pending-tool review (AC5).
pub(crate) const LIST_PENDING_TOOLS_SQL: &str =
    "SELECT * FROM tools WHERE approval_status = 'pending' ORDER BY created_at DESC LIMIT $1";

pub const MAX_ADMIN_REVIEW_LIST_LIMIT: i64 = 100;

pub(crate) fn clamp_admin_review_list_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_ADMIN_REVIEW_LIST_LIMIT)
}

/// Operator review queue identifiers.
pub const REVIEW_QUEUES: &[&str] = &[
    "new_candidate",
    "known_update",
    "needs_manual_research",
    "low_relevance",
    "reported",
    "high_risk_install",
];

/// SQL WHERE fragment for a review queue (testable without DB).
pub(crate) fn review_queue_where(queue: &str) -> Result<&'static str, &'static str> {
    match queue {
        "new_candidate" => Ok(
            "approval_status = 'pending' AND last_reviewed_at IS NULL AND quarantined_at IS NULL",
        ),
        "known_update" => Ok(
            "approval_status = 'approved' AND last_reviewed_at IS NOT NULL \
             AND updated_at > last_reviewed_at AND quarantined_at IS NULL",
        ),
        "needs_manual_research" => Ok(
            "approval_status IN ('pending', 'approved') AND relevance_status = 'needs_review' \
             AND crypto_relevance_score < 50 AND quarantined_at IS NULL",
        ),
        "low_relevance" => Ok(
            "approval_status = 'pending' AND relevance_status = 'rejected' AND quarantined_at IS NULL",
        ),
        "reported" => Ok(
            "id IN (SELECT DISTINCT tool_id FROM tool_reports WHERE status = 'open') \
             AND quarantined_at IS NULL",
        ),
        "high_risk_install" => Ok(
            "approval_status IN ('pending', 'approved') \
             AND install_risk_level IN ('high', 'critical') AND quarantined_at IS NULL",
        ),
        _ => Err("unknown review queue"),
    }
}

pub(crate) fn review_queue_sql(queue: &str) -> Result<&'static str, &'static str> {
    match queue {
        "new_candidate" => Ok("SELECT * FROM tools \
             WHERE approval_status = 'pending' \
               AND last_reviewed_at IS NULL \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        "known_update" => Ok("SELECT * FROM tools \
             WHERE approval_status = 'approved' \
               AND last_reviewed_at IS NOT NULL \
               AND updated_at > last_reviewed_at \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        "needs_manual_research" => Ok("SELECT * FROM tools \
             WHERE approval_status IN ('pending', 'approved') \
               AND relevance_status = 'needs_review' \
               AND crypto_relevance_score < 50 \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        "low_relevance" => Ok("SELECT * FROM tools \
             WHERE approval_status = 'pending' \
               AND relevance_status = 'rejected' \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        "reported" => Ok("SELECT * FROM tools \
             WHERE id IN (SELECT DISTINCT tool_id FROM tool_reports WHERE status = 'open') \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        "high_risk_install" => Ok("SELECT * FROM tools \
             WHERE approval_status IN ('pending', 'approved') \
               AND install_risk_level IN ('high', 'critical') \
               AND quarantined_at IS NULL \
             ORDER BY updated_at DESC \
             LIMIT $1"),
        _ => Err("unknown review queue"),
    }
}

pub(crate) fn review_queue_count_sql(queue: &str) -> Result<&'static str, &'static str> {
    match queue {
        "new_candidate" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE approval_status = 'pending' \
               AND last_reviewed_at IS NULL \
               AND quarantined_at IS NULL"),
        "known_update" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE approval_status = 'approved' \
               AND last_reviewed_at IS NOT NULL \
               AND updated_at > last_reviewed_at \
               AND quarantined_at IS NULL"),
        "needs_manual_research" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE approval_status IN ('pending', 'approved') \
               AND relevance_status = 'needs_review' \
               AND crypto_relevance_score < 50 \
               AND quarantined_at IS NULL"),
        "low_relevance" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE approval_status = 'pending' \
               AND relevance_status = 'rejected' \
               AND quarantined_at IS NULL"),
        "reported" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE id IN (SELECT DISTINCT tool_id FROM tool_reports WHERE status = 'open') \
               AND quarantined_at IS NULL"),
        "high_risk_install" => Ok("SELECT COUNT(*)::bigint FROM tools \
             WHERE approval_status IN ('pending', 'approved') \
               AND install_risk_level IN ('high', 'critical') \
               AND quarantined_at IS NULL"),
        _ => Err("unknown review queue"),
    }
}

/// Stub duplicate candidate surfaced in review rows until dedupe table ships.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateCandidateStub {
    pub slug: String,
    pub name: String,
}

/// Enriched review row for operator console queues.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewQueueItem {
    pub tool: Tool,
    pub duplicate_candidates: Vec<DuplicateCandidateStub>,
    pub lifecycle_state: String,
    pub claim_state: String,
}

/// Derive lifecycle label from tool fields (stub until lifecycle column exists).
pub(crate) fn derive_lifecycle_state(tool: &Tool) -> String {
    if tool.quarantined_at.is_some() {
        return "flagged".into();
    }
    match tool.approval_status.as_str() {
        "approved" => "public_unclaimed".into(),
        "pending" if tool.last_reviewed_at.is_none() => "candidate".into(),
        "pending" => "pending".into(),
        "rejected" => "delisted".into(),
        other => other.into(),
    }
}

/// Claim state from tool row (defaults to unclaimed when empty).
pub(crate) fn derive_claim_state(tool: &Tool) -> String {
    let state = tool.claim_state.trim();
    if state.is_empty() {
        "unclaimed".into()
    } else {
        state.to_string()
    }
}

/// Admin dashboard aggregate counts and crawler health.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDashboardStats {
    pub pending_candidates: i64,
    pub known_updates: i64,
    pub high_risk_installs: i64,
    pub open_reports: i64,
    pub public_tool_count: i64,
    pub needs_manual_research: i64,
    pub low_relevance: i64,
    pub reported: i64,
    pub crawler_sources: Vec<CrawlerSourceView>,
}

/// Count open tool reports; returns 0 when the reports table is not migrated yet.
#[cfg(feature = "ssr")]
async fn count_open_reports(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM tool_reports WHERE status = 'open'")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

/// Count tools with open reports; returns 0 when the reports table is not migrated yet.
#[cfg(feature = "ssr")]
async fn count_reported_tools(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT tool_id)::bigint FROM tool_reports WHERE status = 'open'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

#[cfg(feature = "ssr")]
async fn fetch_duplicate_candidates(
    pool: &sqlx::PgPool,
    tool: &Tool,
) -> Result<Vec<DuplicateCandidateStub>, FnError> {
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
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(&tool.name)
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
            LIMIT 3
            "#,
        )
        .bind(tool.id)
        .bind(repo)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| FnError::new(format!("failed to load duplicate candidates: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(slug, name)| DuplicateCandidateStub { slug, name })
        .collect())
}

/// Gated admin review payload — writes audit events and enforces publication gates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewToolPayload {
    pub slug: String,
    pub action: String,
    pub reason: String,
    pub override_reason: Option<String>,
    pub expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub snapshot_id: Option<uuid::Uuid>,
    pub recommendation_id: Option<uuid::Uuid>,
}

/// Whether a tool has at least one trustworthy publication URL or package id.
pub(crate) fn tool_has_trustworthy_url(tool: &Tool) -> bool {
    let valid_url = |value: &Option<String>| {
        value.as_ref().is_some_and(|u| {
            let trimmed = u.trim();
            trimmed.starts_with("https://") || trimmed.starts_with("http://")
        })
    };
    valid_url(&tool.repo_url)
        || valid_url(&tool.homepage)
        || tool
            .npm_package
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
        || valid_url(&tool.mcp_endpoint)
}

#[cfg(feature = "ssr")]
pub use crate::server::operator_review_transition::{
    review_audit_statuses, review_override_required,
};

/// Validate review action inputs without touching the database.
pub(crate) fn validate_review_action(action: &str, reason: &str) -> Result<(), &'static str> {
    const APPROVAL_ACTIONS: &[&str] = &[
        "approved",
        "rejected",
        "pending",
        "needs_info",
        "quarantine",
        "mark_verified",
        "mark_official",
        "demote_verified",
        "demote_official",
    ];
    if !APPROVAL_ACTIONS.contains(&action) {
        return Err(
            "invalid review action (expected approved|rejected|pending|needs_info|quarantine|mark_verified|mark_official|demote_verified|demote_official)",
        );
    }
    if action == "rejected" && reason.trim().is_empty() {
        return Err("rejection requires a non-empty reason");
    }
    if matches!(
        action,
        "needs_info"
            | "quarantine"
            | "mark_verified"
            | "mark_official"
            | "demote_verified"
            | "demote_official"
    ) && reason.trim().is_empty()
    {
        return Err("review action requires a non-empty reason");
    }
    if action == "approved" && reason.trim().is_empty() {
        return Err("approval requires a non-empty reason");
    }
    Ok(())
}

/// Validate admin approval inputs without touching the database.
pub(crate) fn validate_set_tool_approval_input(
    status: &str,
    reason: Option<&str>,
) -> Result<(), &'static str> {
    let reason_text = reason.map(str::trim).unwrap_or("");
    validate_review_action(
        status,
        if reason_text.is_empty() && status == "approved" {
            "legacy approval"
        } else {
            reason_text
        },
    )
}

/// Core `review_tool` execution inside an open transaction (crate-internal; use `run_review_tool`).
#[cfg(feature = "ssr")]
pub(crate) async fn execute_review_tool_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    admin_id: uuid::Uuid,
    tool: &Tool,
    payload: &ReviewToolPayload,
) -> Result<(), FnError> {
    if let Some(expected) = payload.expected_updated_at {
        if tool.updated_at != expected {
            return Err(FnError::new(
                "tool was modified by another session; refresh and retry",
            ));
        }
    }

    let effect = plan_operator_review(
        tool,
        &payload.action,
        payload.reason.trim(),
        payload.snapshot_id,
    );

    match effect.gate {
        OperatorReviewGate::PublicationApproval => {
            if let Err(msg) =
                validate_review_approval_gate(tool, payload.override_reason.as_deref())
            {
                return Err(FnError::new(msg.to_string()));
            }
        }
        OperatorReviewGate::MarkOfficial => {
            let links = sqlx::query_as::<_, ToolOfficialLink>(
                "SELECT * FROM tool_official_links WHERE tool_id = $1 ORDER BY link_type, created_at",
            )
            .bind(tool.id)
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| FnError::new(format!("failed to load official links: {e}")))?;
            if let Err(msg) = validate_mark_official_gate(tool, &links) {
                return Err(FnError::new(msg.to_string()));
            }
        }
        OperatorReviewGate::DemoteVerified => {
            if let Err(msg) = validate_demote_verified_gate(tool) {
                return Err(FnError::new(msg.to_string()));
            }
        }
        OperatorReviewGate::DemoteOfficial => {
            if let Err(msg) = validate_demote_official_gate(tool) {
                return Err(FnError::new(msg.to_string()));
            }
        }
        OperatorReviewGate::None => {}
    }

    apply_operator_review_in_tx(
        tx,
        admin_id,
        &payload.slug,
        &effect,
        &LegacyReviewEventInput {
            admin_id,
            action: payload.action.clone(),
            reason: payload.reason.clone(),
            override_reason: payload.override_reason.clone(),
            before_status: effect.legacy_audit_before.clone(),
            after_status: effect.legacy_audit_after.clone(),
            snapshot_id: payload.snapshot_id,
            recommendation_id: payload.recommendation_id,
        },
        payload.snapshot_id,
    )
    .await?;

    Ok(())
}

/// Post-auth `review_tool` body — load tool, plan, gate, persist, commit.
#[cfg(feature = "ssr")]
pub async fn run_review_tool(
    pool: &sqlx::PgPool,
    admin_id: uuid::Uuid,
    payload: &ReviewToolPayload,
) -> Result<(), FnError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| FnError::new(format!("failed to start review transaction: {e}")))?;

    let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE slug = $1 FOR UPDATE")
        .bind(&payload.slug)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| FnError::new(format!("failed to load tool: {e}")))?
        .ok_or_else(|| FnError::new(format!("tool not found: {}", payload.slug)))?;

    execute_review_tool_in_tx(&mut tx, admin_id, &tool, payload).await?;

    tx.commit()
        .await
        .map_err(|e| FnError::new(format!("failed to commit review: {e}")))?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ReferralDashboardStats {
    pub x402_tools: i64,
    pub referral_enabled_tools: i64,
    pub attribution_events: i64,
    pub reported_settlements: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateToolReferralPayload {
    pub slug: String,
    pub referral_enabled: bool,
    pub referral_bps: Option<i32>,
    pub referral_payout_address: Option<String>,
    pub referral_model: Option<String>,
    pub x402_pay_to_address: Option<String>,
    pub x402_builder_code: Option<String>,
    pub payment_verified: bool,
    pub x402_endpoint_verified: bool,
    pub price_verified: bool,
}

pub(crate) fn validate_tool_referral_payload(
    payload: &UpdateToolReferralPayload,
) -> Result<(), &'static str> {
    if payload.slug.trim().is_empty() {
        return Err("tool slug is required");
    }
    if let Some(bps) = payload.referral_bps {
        if !(0..=10_000).contains(&bps) {
            return Err("referral bps must be 0–10000");
        }
    }
    if let Some(model) = payload.referral_model.as_deref().map(str::trim) {
        if !model.is_empty() && model != "split" && model != "attribution" {
            return Err("referral model must be split or attribution");
        }
    }
    for value in [
        payload.referral_payout_address.as_deref(),
        payload.x402_pay_to_address.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.trim().len() > 200 {
            return Err("referral and pay-to addresses must be 200 characters or fewer");
        }
    }
    if let Some(code) = payload.x402_builder_code.as_deref() {
        if code.trim().len() > 100 {
            return Err("x402 builder code must be 100 characters or fewer");
        }
    }
    Ok(())
}
