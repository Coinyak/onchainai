//! Review harness persistence — official links, review runs, entries, operator verdicts.

use crate::models::{OperatorVerdict, ReviewEntry, ReviewRun, Tool, ToolOfficialLink};
use crate::server::fn_error::FnError;
use crate::server::operator_review_transition::{OperatorReviewEffect, ToolReviewSqlUpdate};
use crate::trust_verification::{
    official_promotion_allowed, verify_tool_trust, TrustVerificationResult,
};

#[derive(Debug, Clone)]
pub struct InsertReviewRunInput {
    pub tool_id: uuid::Uuid,
    pub queue: Option<String>,
    pub runner_name: String,
    pub prompt_version: Option<String>,
    pub snapshot_version: String,
    pub created_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct InsertReviewEntryInput {
    pub review_run_id: uuid::Uuid,
    pub entry_type: String,
    pub role: String,
    pub agent_label: Option<String>,
    pub recommended_action: Option<String>,
    pub confidence: Option<f32>,
    pub rationale: Option<String>,
    pub supporting_evidence_json: serde_json::Value,
    pub dissent_json: serde_json::Value,
    pub missing_proofs_json: serde_json::Value,
}

/// Verdict row to append after `review_tool` has already updated the tool.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertOperatorVerdictInput {
    pub tool_id: uuid::Uuid,
    pub review_run_id: Option<uuid::Uuid>,
    pub action: String,
    pub from_status: String,
    pub to_status: String,
    pub from_claim_state: String,
    pub to_claim_state: Option<String>,
    pub reason_codes: Vec<String>,
    pub note: Option<String>,
}

/// Server-side gate for mark-official — human operator still required after this passes.
pub fn validate_mark_official_gate(
    tool: &Tool,
    official_links: &[ToolOfficialLink],
) -> Result<(), &'static str> {
    let trust = verify_tool_trust(tool, official_links);
    if official_promotion_allowed(tool, official_links, &trust) {
        Ok(())
    } else {
        Err("mark official requires claimed status and at least two strongly verified official links")
    }
}

#[derive(Debug, Clone)]
pub struct VerifyOfficialLinkInput {
    pub link_id: uuid::Uuid,
    pub verification_status: String,
    pub evidence_strength: String,
    pub official_badge_allowed: bool,
    pub verification_method: Option<String>,
    pub notes: Option<String>,
    pub operator_id: uuid::Uuid,
}

pub async fn insert_review_run(
    pool: &sqlx::PgPool,
    input: InsertReviewRunInput,
) -> Result<ReviewRun, FnError> {
    sqlx::query_as::<_, ReviewRun>(
        r#"
        INSERT INTO review_runs (
            tool_id, queue, runner_name, prompt_version, snapshot_version, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.tool_id)
    .bind(input.queue)
    .bind(input.runner_name)
    .bind(input.prompt_version)
    .bind(input.snapshot_version)
    .bind(input.created_by)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to create review run: {e}")))
}

pub async fn complete_review_run(
    pool: &sqlx::PgPool,
    run_id: uuid::Uuid,
    summary: Option<String>,
    status: &str,
) -> Result<ReviewRun, FnError> {
    sqlx::query_as::<_, ReviewRun>(
        r#"
        UPDATE review_runs
        SET status = $2, summary = $3, completed_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(run_id)
    .bind(status)
    .bind(summary)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to complete review run: {e}")))
}

pub async fn insert_review_entry(
    pool: &sqlx::PgPool,
    input: InsertReviewEntryInput,
) -> Result<ReviewEntry, FnError> {
    sqlx::query_as::<_, ReviewEntry>(
        r#"
        INSERT INTO review_entries (
            review_run_id, entry_type, role, agent_label, recommended_action,
            confidence, rationale, supporting_evidence_json, dissent_json, missing_proofs_json
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(input.review_run_id)
    .bind(input.entry_type)
    .bind(input.role)
    .bind(input.agent_label)
    .bind(input.recommended_action)
    .bind(input.confidence)
    .bind(input.rationale)
    .bind(input.supporting_evidence_json)
    .bind(input.dissent_json)
    .bind(input.missing_proofs_json)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to append review entry: {e}")))
}

pub async fn list_review_entries(
    pool: &sqlx::PgPool,
    review_run_id: uuid::Uuid,
) -> Result<Vec<ReviewEntry>, FnError> {
    sqlx::query_as::<_, ReviewEntry>(
        "SELECT * FROM review_entries WHERE review_run_id = $1 ORDER BY created_at ASC",
    )
    .bind(review_run_id)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to list review entries: {e}")))
}

/// Review timeline entries (agent reviews + operator notes). Verdict rows are separate.
pub async fn list_tool_review_entries(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<Vec<ReviewEntry>, FnError> {
    sqlx::query_as::<_, ReviewEntry>(
        r#"
        SELECT e.*
        FROM review_entries e
        JOIN review_runs r ON r.id = e.review_run_id
        WHERE r.tool_id = $1
        ORDER BY e.created_at DESC
        LIMIT 100
        "#,
    )
    .bind(tool_id)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to load review entries: {e}")))
}

/// Back-compat alias for harness callers.
pub async fn list_tool_review_timeline(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<Vec<ReviewEntry>, FnError> {
    list_tool_review_entries(pool, tool_id).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ToolReviewTimelineBundle {
    pub entries: Vec<ReviewEntry>,
    pub operator_verdicts: Vec<OperatorVerdict>,
}

pub async fn load_tool_review_timeline(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<ToolReviewTimelineBundle, FnError> {
    let entries = list_tool_review_entries(pool, tool_id).await?;
    let operator_verdicts = list_operator_verdicts(pool, tool_id).await?;
    Ok(ToolReviewTimelineBundle {
        entries,
        operator_verdicts,
    })
}

pub async fn list_operator_verdicts(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<Vec<OperatorVerdict>, FnError> {
    sqlx::query_as::<_, OperatorVerdict>(
        "SELECT * FROM operator_verdicts WHERE tool_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(tool_id)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to list operator verdicts: {e}")))
}

/// Statuses safe for public tool detail — operator-verified only (no user-submitted candidates).
pub const PUBLIC_OFFICIAL_LINK_STATUSES: &[&str] = &["claimed", "verified"];

pub fn is_public_official_link(link: &ToolOfficialLink) -> bool {
    PUBLIC_OFFICIAL_LINK_STATUSES.contains(&link.verification_status.as_str())
}

pub async fn list_official_links(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<Vec<ToolOfficialLink>, FnError> {
    sqlx::query_as::<_, ToolOfficialLink>(
        "SELECT * FROM tool_official_links WHERE tool_id = $1 ORDER BY link_type, created_at",
    )
    .bind(tool_id)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to load official links: {e}")))
}

pub async fn list_public_official_links(
    pool: &sqlx::PgPool,
    tool_id: uuid::Uuid,
) -> Result<Vec<ToolOfficialLink>, FnError> {
    sqlx::query_as::<_, ToolOfficialLink>(
        r#"
        SELECT * FROM tool_official_links
        WHERE tool_id = $1
          AND verification_status = ANY($2::text[])
        ORDER BY link_type, created_at
        "#,
    )
    .bind(tool_id)
    .bind(PUBLIC_OFFICIAL_LINK_STATUSES)
    .fetch_all(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to load public official links: {e}")))
}

pub async fn insert_candidate_official_link(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
    link_type: &str,
    url: &str,
    display_label: &str,
    discovered_from: &str,
) -> Result<(), FnError> {
    sqlx::query(
        r#"
        INSERT INTO tool_official_links (
            tool_id, link_type, url, display_label, verification_status,
            official_badge_allowed, evidence_strength, discovered_from
        )
        VALUES ($1, $2, $3, $4, 'candidate', false, 'weak', $5)
        ON CONFLICT (tool_id, link_type, url) DO NOTHING
        "#,
    )
    .bind(tool_id)
    .bind(link_type)
    .bind(url)
    .bind(display_label)
    .bind(discovered_from)
    .execute(&mut **tx)
    .await
    .map_err(|e| FnError::new(format!("failed to insert official link: {e}")))?;
    Ok(())
}

pub async fn verify_official_link(
    pool: &sqlx::PgPool,
    input: VerifyOfficialLinkInput,
) -> Result<ToolOfficialLink, FnError> {
    sqlx::query_as::<_, ToolOfficialLink>(
        r#"
        UPDATE tool_official_links
        SET verification_status = $2,
            evidence_strength = $3,
            official_badge_allowed = $4,
            verification_method = $5,
            notes = $6,
            verified_by = $7,
            verified_at = now(),
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(input.link_id)
    .bind(input.verification_status)
    .bind(input.evidence_strength)
    .bind(input.official_badge_allowed)
    .bind(input.verification_method)
    .bind(input.notes)
    .bind(input.operator_id)
    .fetch_one(pool)
    .await
    .map_err(|e| FnError::new(format!("failed to verify official link: {e}")))
}

pub async fn compute_tool_trust(
    pool: &sqlx::PgPool,
    tool: &Tool,
) -> Result<(TrustVerificationResult, Vec<ToolOfficialLink>), FnError> {
    let links = list_official_links(pool, tool.id).await?;
    let trust = verify_tool_trust(tool, &links);
    Ok((trust, links))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorReviewRunStrategy {
    CreateOperatorRun,
    ReuseHarnessRun,
}

fn resolve_operator_review_run_strategy(
    harness_run_id: Option<uuid::Uuid>,
) -> OperatorReviewRunStrategy {
    if harness_run_id.is_some() {
        OperatorReviewRunStrategy::ReuseHarnessRun
    } else {
        OperatorReviewRunStrategy::CreateOperatorRun
    }
}

#[derive(Debug, Clone)]
pub struct LegacyReviewEventInput {
    pub admin_id: uuid::Uuid,
    pub action: String,
    pub reason: String,
    pub override_reason: Option<String>,
    pub before_status: String,
    pub after_status: String,
    pub snapshot_id: Option<uuid::Uuid>,
    pub recommendation_id: Option<uuid::Uuid>,
}

async fn apply_tool_review_update_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    slug: &str,
    update: &ToolReviewSqlUpdate,
) -> Result<(), FnError> {
    if update.quarantine {
        sqlx::query(
            r#"
            UPDATE tools
            SET quarantined_at = now(),
                last_reviewed_at = now(),
                updated_at = now()
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .execute(&mut **tx)
        .await
        .map_err(|e| FnError::new(format!("failed to quarantine tool: {e}")))?;
        return Ok(());
    }

    if let Some(ref listing_status) = update.listing_status {
        sqlx::query(
            r#"
            UPDATE tools
            SET status = $1,
                last_reviewed_at = now(),
                updated_at = now()
            WHERE slug = $2
            "#,
        )
        .bind(listing_status)
        .bind(slug)
        .execute(&mut **tx)
        .await
        .map_err(|e| FnError::new(format!("failed to update listing status: {e}")))?;
        if let Some(ref claim_state) = update.claim_state {
            sqlx::query("UPDATE tools SET claim_state = $1, updated_at = now() WHERE slug = $2")
                .bind(claim_state)
                .bind(slug)
                .execute(&mut **tx)
                .await
                .map_err(|e| FnError::new(format!("failed to update claim state: {e}")))?;
        }
        return Ok(());
    }

    if let Some(ref approval_status) = update.approval_status {
        let rejection_reason = update.rejection_reason.as_ref().and_then(|r| r.clone());
        if let Some(ref relevance_status) = update.relevance_status {
            sqlx::query(
                r#"
                UPDATE tools
                SET approval_status = $1,
                    rejection_reason = $2,
                    relevance_status = $3,
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $4
                "#,
            )
            .bind(approval_status)
            .bind(rejection_reason)
            .bind(relevance_status)
            .bind(slug)
            .execute(&mut **tx)
            .await
            .map_err(|e| FnError::new(format!("failed to update approval: {e}")))?;
        } else {
            sqlx::query(
                r#"
                UPDATE tools
                SET approval_status = $1,
                    rejection_reason = $2,
                    last_reviewed_at = now(),
                    updated_at = now()
                WHERE slug = $3
                "#,
            )
            .bind(approval_status)
            .bind(rejection_reason)
            .bind(slug)
            .execute(&mut **tx)
            .await
            .map_err(|e| FnError::new(format!("failed to update approval: {e}")))?;
        }
        if let Some(ref claim_state) = update.claim_state {
            sqlx::query("UPDATE tools SET claim_state = $1, updated_at = now() WHERE slug = $2")
                .bind(claim_state)
                .bind(slug)
                .execute(&mut **tx)
                .await
                .map_err(|e| FnError::new(format!("failed to update claim state: {e}")))?;
        }
        return Ok(());
    }

    Ok(())
}

/// Apply tool mutation, legacy audit row, verdict, and timeline note in one transaction.
pub async fn apply_operator_review_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operator_id: uuid::Uuid,
    slug: &str,
    effect: &OperatorReviewEffect,
    legacy: &LegacyReviewEventInput,
    harness_run_id: Option<uuid::Uuid>,
) -> Result<(OperatorVerdict, ReviewEntry), FnError> {
    sqlx::query(
        r#"
        INSERT INTO tool_review_events (
            tool_id, admin_id, action, reason, override_reason,
            before_status, after_status, snapshot_id, recommendation_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(effect.verdict.tool_id)
    .bind(legacy.admin_id)
    .bind(&legacy.action)
    .bind(legacy.reason.trim())
    .bind(legacy.override_reason.as_deref().map(str::trim))
    .bind(&legacy.before_status)
    .bind(&legacy.after_status)
    .bind(legacy.snapshot_id)
    .bind(legacy.recommendation_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| FnError::new(format!("failed to write review event: {e}")))?;

    apply_tool_review_update_in_tx(tx, slug, &effect.tool_update).await?;

    record_operator_review_action_in_tx(
        tx,
        effect.verdict.tool_id,
        operator_id,
        &effect.verdict.action,
        legacy.reason.trim(),
        harness_run_id,
        effect.verdict.clone(),
    )
    .await
}

/// Atomically record operator verdict + timeline note on one review run.
pub async fn record_operator_review_action_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tool_id: uuid::Uuid,
    operator_id: uuid::Uuid,
    action: &str,
    note: &str,
    harness_run_id: Option<uuid::Uuid>,
    mut verdict_input: InsertOperatorVerdictInput,
) -> Result<(OperatorVerdict, ReviewEntry), FnError> {
    let run_id = match resolve_operator_review_run_strategy(harness_run_id) {
        OperatorReviewRunStrategy::ReuseHarnessRun => {
            harness_run_id.expect("reuse strategy requires harness run id")
        }
        OperatorReviewRunStrategy::CreateOperatorRun => {
            let run = sqlx::query_as::<_, ReviewRun>(
                r#"
                INSERT INTO review_runs (
                    tool_id, queue, runner_name, snapshot_version, status, summary, created_by, completed_at
                )
                VALUES ($1, 'operator_action', 'operator', 'operator-note-v1', 'completed', $2, $3, now())
                RETURNING *
                "#,
            )
            .bind(tool_id)
            .bind(format!("Operator action: {action}"))
            .bind(operator_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| FnError::new(format!("failed to create operator review run: {e}")))?;
            run.id
        }
    };

    verdict_input.review_run_id = Some(run_id);
    let verdict = insert_operator_verdict_in_tx(tx, operator_id, verdict_input).await?;

    let entry = sqlx::query_as::<_, ReviewEntry>(
        r#"
        INSERT INTO review_entries (
            review_run_id, entry_type, role, agent_label, recommended_action,
            rationale, supporting_evidence_json, dissent_json, missing_proofs_json
        )
        VALUES ($1, 'operator_note', 'operator_note', $2, $3, $4, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb)
        RETURNING *
        "#,
    )
    .bind(run_id)
    .bind(operator_id.to_string())
    .bind(action)
    .bind(note)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| FnError::new(format!("failed to append operator note: {e}")))?;

    Ok((verdict, entry))
}

pub async fn insert_operator_verdict_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operator_id: uuid::Uuid,
    input: InsertOperatorVerdictInput,
) -> Result<OperatorVerdict, FnError> {
    sqlx::query_as::<_, OperatorVerdict>(
        r#"
        INSERT INTO operator_verdicts (
            tool_id, review_run_id, action, from_status, to_status,
            from_claim_state, to_claim_state, reason_codes, note, operator_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(input.tool_id)
    .bind(input.review_run_id)
    .bind(&input.action)
    .bind(&input.from_status)
    .bind(&input.to_status)
    .bind(&input.from_claim_state)
    .bind(&input.to_claim_state)
    .bind(&input.reason_codes)
    .bind(&input.note)
    .bind(operator_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| FnError::new(format!("failed to insert operator verdict: {e}")))
}

#[cfg(test)]
#[path = "review_persistence_tests.rs"]
mod tests;
