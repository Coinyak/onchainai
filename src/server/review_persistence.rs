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
mod tests {
    use super::*;
    use crate::models::tool::default_review_fields;

    fn tool_with_claim(claim_state: &str) -> Tool {
        let review = default_review_fields();
        Tool {
            id: uuid::Uuid::nil(),
            name: "Demo".into(),
            slug: "demo".into(),
            description: None,
            function: "dev-tool".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "mcp".into(),
            repo_url: Some("https://github.com/org/repo".into()),
            homepage: Some("https://example.com".into()),
            npm_package: Some("@org/pkg".into()),
            install_command: Some("npx @org/pkg".into()),
            mcp_endpoint: None,
            chains: vec![],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 80,
            crypto_relevance_reasons: vec![],
            relevance_status: "accepted".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec![],
            requires_secret: false,
            safe_copy_command: None,
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: claim_state.into(),
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
            last_commit_at: Some(chrono::Utc::now()),
            source: "manual".into(),
            source_url: None,
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn verified_link(link_type: &str) -> ToolOfficialLink {
        ToolOfficialLink {
            id: uuid::Uuid::new_v4(),
            tool_id: uuid::Uuid::nil(),
            link_type: link_type.into(),
            url: "https://example.com".into(),
            display_label: "Official".into(),
            verification_status: "verified".into(),
            official_badge_allowed: true,
            evidence_strength: "strong".into(),
            verification_method: Some("operator_review".into()),
            discovered_from: None,
            verified_by: None,
            verified_at: None,
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn is_public_official_link_excludes_rejected_status() {
        let mut link = verified_link("github");
        assert!(is_public_official_link(&link));
        link.verification_status = "rejected".into();
        assert!(!is_public_official_link(&link));
    }

    #[test]
    fn validate_mark_official_gate_blocks_unclaimed_tool() {
        let tool = tool_with_claim("unclaimed");
        let links = vec![verified_link("github"), verified_link("website")];
        assert!(validate_mark_official_gate(&tool, &links).is_err());
    }

    #[test]
    fn validate_mark_official_gate_requires_two_strong_verified_links() {
        let tool = tool_with_claim("claimed");
        let links = vec![verified_link("github")];
        assert!(validate_mark_official_gate(&tool, &links).is_err());
    }

    #[test]
    fn validate_mark_official_gate_passes_with_claim_and_two_verified_links() {
        let tool = tool_with_claim("claimed");
        let links = vec![verified_link("github"), verified_link("website")];
        assert!(validate_mark_official_gate(&tool, &links).is_ok());
    }

    async fn test_pool() -> Option<sqlx::PgPool> {
        let database_url = std::env::var("SUPABASE_URL_TEST")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()?;
        sqlx::PgPool::connect(&database_url).await.ok()
    }

    #[tokio::test]
    async fn apply_operator_review_in_tx_persists_linked_verdict_and_entry() {
        let Some(pool) = test_pool().await else {
            eprintln!(
                "SKIP: SUPABASE_URL_TEST or DATABASE_URL not set — apply_operator_review_in_tx DB test"
            );
            return;
        };

        let mut tx = pool
            .begin()
            .await
            .expect("begin transaction for apply_operator_review_in_tx test");

        let operator_id = uuid::Uuid::new_v4();
        let nickname = format!("op-{}", operator_id.as_simple());
        sqlx::query(
            "INSERT INTO profiles (id, nickname, auth_method, is_admin) VALUES ($1, $2, 'email', true)",
        )
        .bind(operator_id)
        .bind(&nickname)
        .execute(&mut *tx)
        .await
        .expect("insert operator profile");

        let slug = format!("apply-review-test-{}", uuid::Uuid::new_v4());
        let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, chains,
                status, trust_score, approval_status, claim_state, source, source_url
            )
            VALUES (
                'Apply Review Test', $1, 'test', 'dev-tool', 'crypto',
                'human', 'mcp', 'https://github.com/org/repo', 'https://example.com',
                '@org/pkg', 'npx @org/pkg', ARRAY[]::text[],
                'community', 0, 'approved', 'unclaimed', 'manual', 'https://example.com'
            )
            RETURNING id
            "#,
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .expect("insert test tool");

        let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("load test tool");

        let effect = crate::server::operator_review_transition::plan_operator_review(
            &tool,
            "mark_verified",
            "operator verified install path",
            None,
        );

        let (verdict, entry) = apply_operator_review_in_tx(
            &mut tx,
            operator_id,
            &slug,
            &effect,
            &LegacyReviewEventInput {
                admin_id: operator_id,
                action: "mark_verified".into(),
                reason: "operator verified install path".into(),
                override_reason: None,
                before_status: effect.legacy_audit_before.clone(),
                after_status: effect.legacy_audit_after.clone(),
                snapshot_id: None,
                recommendation_id: None,
            },
            None,
        )
        .await
        .expect("apply_operator_review_in_tx");

        assert_eq!(verdict.tool_id, tool_id);
        assert_eq!(verdict.action, "mark_verified");
        assert_eq!(entry.entry_type, "operator_note");
        assert_eq!(verdict.review_run_id, Some(entry.review_run_id));
        assert!(verdict.review_run_id.is_some());

        let status: String = sqlx::query_scalar("SELECT status FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("read updated listing status");
        assert_eq!(status, "verified");

        tx.rollback()
            .await
            .expect("rollback apply_operator_review_in_tx test");
    }

    #[tokio::test]
    async fn apply_operator_review_in_tx_approves_claim_pending_into_claimed() {
        let Some(pool) = test_pool().await else {
            eprintln!("SKIP: SUPABASE_URL_TEST or DATABASE_URL not set — claim_pending DB test");
            return;
        };

        let mut tx = pool.begin().await.expect("begin claim_pending test tx");

        let operator_id = uuid::Uuid::new_v4();
        let nickname = format!("op-claim-{}", operator_id.as_simple());
        sqlx::query(
            "INSERT INTO profiles (id, nickname, auth_method, is_admin) VALUES ($1, $2, 'email', true)",
        )
        .bind(operator_id)
        .bind(&nickname)
        .execute(&mut *tx)
        .await
        .expect("insert operator profile");

        let slug = format!("claim-pending-test-{}", uuid::Uuid::new_v4());
        let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, chains,
                status, trust_score, approval_status, claim_state, relevance_status,
                source, source_url
            )
            VALUES (
                'Claim Pending Test', $1, 'test', 'dev-tool', 'crypto',
                'human', 'mcp', 'https://github.com/org/repo', 'https://example.com',
                '@org/pkg', 'npx @org/pkg', ARRAY[]::text[],
                'community', 0, 'pending', 'claim_pending', 'accepted',
                'manual', 'https://example.com'
            )
            RETURNING id
            "#,
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .expect("insert claim_pending tool");

        let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("load claim_pending tool");
        assert_eq!(tool.claim_state, "claim_pending");

        let effect = crate::server::operator_review_transition::plan_operator_review(
            &tool,
            "approved",
            "claim proof verified by operator",
            None,
        );
        assert_eq!(effect.tool_update.claim_state.as_deref(), Some("claimed"));

        let (verdict, entry) = apply_operator_review_in_tx(
            &mut tx,
            operator_id,
            &slug,
            &effect,
            &LegacyReviewEventInput {
                admin_id: operator_id,
                action: "approved".into(),
                reason: "claim proof verified by operator".into(),
                override_reason: None,
                before_status: effect.legacy_audit_before.clone(),
                after_status: effect.legacy_audit_after.clone(),
                snapshot_id: None,
                recommendation_id: None,
            },
            None,
        )
        .await
        .expect("apply approved claim_pending review");

        assert_eq!(verdict.action, "approved");
        assert_eq!(verdict.from_claim_state.as_deref(), Some("claim_pending"));
        assert_eq!(verdict.to_claim_state.as_deref(), Some("claimed"));
        assert_eq!(verdict.review_run_id, Some(entry.review_run_id));

        let claim_state: String = sqlx::query_scalar("SELECT claim_state FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("read updated claim_state");
        assert_eq!(claim_state, "claimed");

        let approval_status: String =
            sqlx::query_scalar("SELECT approval_status FROM tools WHERE id = $1")
                .bind(tool_id)
                .fetch_one(&mut *tx)
                .await
                .expect("read updated approval_status");
        assert_eq!(approval_status, "approved");

        tx.rollback().await.expect("rollback claim_pending test");
    }

    #[tokio::test]
    async fn apply_operator_review_in_tx_promotes_claimed_tool_to_official() {
        let Some(pool) = test_pool().await else {
            eprintln!("SKIP: DATABASE_URL not set — mark_official promotion DB test");
            return;
        };

        let mut tx = pool.begin().await.expect("begin mark_official chain tx");

        let operator_id = uuid::Uuid::new_v4();
        let nickname = format!("op-official-{}", operator_id.as_simple());
        sqlx::query(
            "INSERT INTO profiles (id, nickname, auth_method, is_admin) VALUES ($1, $2, 'email', true)",
        )
        .bind(operator_id)
        .bind(&nickname)
        .execute(&mut *tx)
        .await
        .expect("insert operator profile");

        let slug = format!("official-chain-test-{}", uuid::Uuid::new_v4());
        let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, chains,
                status, trust_score, approval_status, claim_state, relevance_status,
                last_commit_at, source, source_url
            )
            VALUES (
                'Official Chain Test', $1, 'test', 'dev-tool', 'crypto',
                'human', 'mcp', 'https://github.com/org/repo', 'https://example.com',
                '@org/pkg', 'npx @org/pkg', ARRAY[]::text[],
                'community', 0, 'pending', 'claim_pending', 'accepted',
                now(), 'manual', 'https://example.com'
            )
            RETURNING id
            "#,
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .expect("insert claim_pending tool for official chain");

        let mut tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("load tool");

        let approve_effect = crate::server::operator_review_transition::plan_operator_review(
            &tool,
            "approved",
            "claim proof accepted",
            None,
        );
        apply_operator_review_in_tx(
            &mut tx,
            operator_id,
            &slug,
            &approve_effect,
            &LegacyReviewEventInput {
                admin_id: operator_id,
                action: "approved".into(),
                reason: "claim proof accepted".into(),
                override_reason: None,
                before_status: approve_effect.legacy_audit_before.clone(),
                after_status: approve_effect.legacy_audit_after.clone(),
                snapshot_id: None,
                recommendation_id: None,
            },
            None,
        )
        .await
        .expect("approve claim_pending in official chain");

        tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("reload tool after approval");
        assert_eq!(tool.claim_state, "claimed");
        assert_eq!(tool.approval_status, "approved");

        for (link_type, url) in [
            ("github", "https://github.com/org/repo"),
            ("website", "https://example.com"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO tool_official_links (
                    tool_id, link_type, url, display_label, verification_status,
                    official_badge_allowed, evidence_strength, verification_method, verified_by
                )
                VALUES ($1, $2, $3, 'Official', 'verified', true, 'strong', 'operator_review', $4)
                "#,
            )
            .bind(tool_id)
            .bind(link_type)
            .bind(url)
            .bind(operator_id)
            .execute(&mut *tx)
            .await
            .expect("insert verified official link");
        }

        let links = sqlx::query_as::<_, ToolOfficialLink>(
            "SELECT * FROM tool_official_links WHERE tool_id = $1 ORDER BY link_type",
        )
        .bind(tool_id)
        .fetch_all(&mut *tx)
        .await
        .expect("load official links");
        assert!(validate_mark_official_gate(&tool, &links).is_ok());

        let official_effect = crate::server::operator_review_transition::plan_operator_review(
            &tool,
            "mark_official",
            "two strongly verified official links on file",
            None,
        );
        let (verdict, _) = apply_operator_review_in_tx(
            &mut tx,
            operator_id,
            &slug,
            &official_effect,
            &LegacyReviewEventInput {
                admin_id: operator_id,
                action: "mark_official".into(),
                reason: "two strongly verified official links on file".into(),
                override_reason: None,
                before_status: official_effect.legacy_audit_before.clone(),
                after_status: official_effect.legacy_audit_after.clone(),
                snapshot_id: None,
                recommendation_id: None,
            },
            None,
        )
        .await
        .expect("mark_official after claim transition");

        assert_eq!(verdict.action, "mark_official");
        let listing_status: String = sqlx::query_scalar("SELECT status FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_one(&mut *tx)
            .await
            .expect("read official listing status");
        assert_eq!(listing_status, "official");

        tx.rollback()
            .await
            .expect("rollback mark_official chain test");
    }
}
