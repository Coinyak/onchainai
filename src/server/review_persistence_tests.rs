//! Tests extracted from `review_persistence.rs` for Code Health scoring.

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
