//! Review harness flow tests — pure transition wiring + optional DB persistence.

use onchainai::models::tool::default_review_fields;
use onchainai::models::Tool;
use onchainai::server::operator_review_transition::{plan_operator_review, OperatorReviewGate};

fn harness_tool(claim_state: &str, approval: &str) -> Tool {
    let review = default_review_fields();
    Tool {
        id: uuid::Uuid::new_v4(),
        name: "Harness".into(),
        slug: format!("harness-{}", uuid::Uuid::new_v4()),
        description: None,
        function: "dev-tool".into(),
        asset_class: "crypto".into(),
        actor: "human".into(),
        tool_type: "mcp".into(),
        repo_url: Some("https://github.com/org/repo".into()),
        homepage: Some("https://example.com".into()),
        npm_package: None,
        install_command: None,
        mcp_endpoint: None,
        chains: vec![],
        status: "community".into(),
        official_team: None,
        trust_score: 0,
        approval_status: approval.into(),
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
        stars: 0,
        last_commit_at: None,
        source: "manual".into(),
        source_url: None,
        logo_url: None,
        logo_monogram: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn harness_operator_verdict_effect_requires_human_run_when_no_snapshot() {
    let tool = harness_tool("claim_pending", "pending");
    let effect = plan_operator_review(&tool, "approved", "operator confirmed claim proof", None);
    assert_eq!(effect.gate, OperatorReviewGate::PublicationApproval);
    assert!(effect.verdict.review_run_id.is_none());
    assert_eq!(effect.verdict.to_claim_state.as_deref(), Some("claimed"));
}

#[test]
fn harness_operator_verdict_effect_links_to_external_review_run() {
    let harness_run = uuid::Uuid::new_v4();
    let tool = harness_tool("claimed", "approved");
    let effect = plan_operator_review(
        &tool,
        "mark_verified",
        "install command verified",
        Some(harness_run),
    );
    assert_eq!(effect.verdict.review_run_id, Some(harness_run));
}

#[cfg(feature = "ssr")]
mod ssr {
    use onchainai::server::review_persistence::{
        insert_review_entry, insert_review_run, list_review_entries, InsertReviewEntryInput,
        InsertReviewRunInput,
    };

    async fn test_pool() -> Option<sqlx::PgPool> {
        let database_url = std::env::var("SUPABASE_URL_TEST")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()?;
        sqlx::PgPool::connect(&database_url).await.ok()
    }

    #[tokio::test]
    async fn harness_run_persists_review_entries_and_requires_human_verdict() {
        let Some(pool) = test_pool().await else {
            eprintln!(
                "SKIP: SUPABASE_URL_TEST or DATABASE_URL not set — harness DB persistence not exercised"
            );
            return;
        };

        let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO tools (
                name, slug, description, function, asset_class, actor, type,
                repo_url, homepage, npm_package, install_command, chains,
                status, trust_score, approval_status, source, source_url
            )
            VALUES (
                'BOB Gateway CLI', $1, 'Bridge CLI', 'bridge', 'crypto',
                'human', 'cli', 'https://github.com/bob-collective/bob', 'https://gobob.xyz',
                '@gobob/gateway-cli', 'npx @gobob/gateway-cli', ARRAY['bitcoin', 'base'],
                'community', 0, 'approved', 'npm', 'https://www.npmjs.com/package/@gobob/gateway-cli'
            )
            RETURNING id
            "#,
        )
        .bind(format!(
            "bob-gateway-cli-review-test-{}",
            uuid::Uuid::new_v4()
        ))
        .fetch_one(&pool)
        .await
        .expect("insert tool");

        let review_run = insert_review_run(
            &pool,
            InsertReviewRunInput {
                tool_id,
                queue: Some("claim_pending".into()),
                runner_name: "codex".into(),
                prompt_version: Some("review-v1".into()),
                snapshot_version: "operator-snapshot-v2".into(),
                created_by: None,
            },
        )
        .await
        .expect("create review run");

        insert_review_entry(
            &pool,
            InsertReviewEntryInput {
                review_run_id: review_run.id,
                entry_type: "agent_review".into(),
                role: "identity".into(),
                agent_label: Some("codex-identity-1".into()),
                recommended_action: Some("request_claim_proof".into()),
                confidence: Some(0.77),
                rationale: Some("GitHub and website align, but official X proof missing".into()),
                supporting_evidence_json: serde_json::json!([{
                    "source": "github",
                    "detail": "repo/homepage aligned"
                }]),
                dissent_json: serde_json::json!([]),
                missing_proofs_json: serde_json::json!(["site backlink to official X"]),
            },
        )
        .await
        .expect("append review entry");

        let timeline = list_review_entries(&pool, review_run.id)
            .await
            .expect("load timeline");

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].role, "identity");
        assert_eq!(
            timeline[0].recommended_action.as_deref(),
            Some("request_claim_proof")
        );
        assert!(timeline[0].confidence.unwrap() > 0.7);

        let _ = sqlx::query("DELETE FROM tools WHERE id = $1")
            .bind(tool_id)
            .execute(&pool)
            .await;
    }
}
