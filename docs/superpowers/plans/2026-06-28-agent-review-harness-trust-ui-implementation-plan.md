# Agent Review Harness Trust UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first full-stack version of the operator review workbench, public trust UI, submit/claim proof flow, and model-agnostic agent review harness using real persisted review data.

**Architecture:** Extend the existing `tools`, claim, queue, and operator harness foundations instead of replacing them. Add dedicated tables for official links, agent review runs, review timeline entries, and operator verdicts; compute trust facts server-side; then surface them across `/admin/tools`, `/tools/:slug`, and `/submit`.

**Tech Stack:** Rust, Leptos SSR, Axum server functions, sqlx/Postgres migrations, Tailwind/CSS, existing operator harness endpoints, Playwright smoke/screenshot checks.

---

## File Map

### Existing files to modify

- `src/lib.rs`
- `src/models/mod.rs`
- `src/models/submission.rs`
- `src/models/tool.rs`
- `src/server/functions.rs`
- `src/server/operator_harness.rs`
- `src/pages/admin/tools.rs`
- `src/pages/tool_detail.rs`
- `src/pages/submit.rs`
- `src/components/tool_detail_content.rs`
- `src/components/tool_card.rs`

### New files to create

- `migrations/019_agent_review_harness.sql`
- `src/models/review.rs`
- `src/trust_verification.rs`
- `src/components/admin_review_timeline.rs`
- `src/components/admin_review_decision_panel.rs`
- `src/components/tool_trust_facts.rs`
- `src/components/official_links_list.rs`
- `src/components/claim_status_timeline.rs`
- `tests/agent_review_harness_flow.rs`

### Existing files likely to receive tests

- `src/server/functions.rs`
- `src/server/operator_harness.rs`
- `src/trust_verification.rs`
- `src/models/review.rs`

---

### Task 1: Add Review-Harness Schema And Rust Models

**Files:**
- Create: `migrations/019_agent_review_harness.sql`
- Create: `src/models/review.rs`
- Modify: `src/models/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/models/review.rs`

- [ ] **Step 1: Write the failing model/serde tests**

Add tests that describe the new persisted review objects before creating them.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_link_verification_status_round_trips() {
        let row = ToolOfficialLink {
            id: uuid::Uuid::nil(),
            tool_id: uuid::Uuid::nil(),
            link_type: "github".into(),
            url: "https://github.com/bob-collective/bob".into(),
            display_label: "Official GitHub".into(),
            verification_status: "verified".into(),
            official_badge_allowed: true,
            evidence_strength: "strong".into(),
            verification_method: Some("site_backlink".into()),
            discovered_from: Some("crawler:npm".into()),
            verified_by: None,
            verified_at: None,
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&row).expect("serialize official link");
        let round_trip: ToolOfficialLink = serde_json::from_str(&json).expect("deserialize official link");
        assert_eq!(round_trip.link_type, "github");
        assert!(round_trip.official_badge_allowed);
    }

    #[test]
    fn review_entry_preserves_role_and_action() {
        let row = ReviewEntry {
            id: uuid::Uuid::nil(),
            review_run_id: uuid::Uuid::nil(),
            entry_type: "agent_review".into(),
            role: "critic".into(),
            agent_label: Some("codex-critic-1".into()),
            recommended_action: Some("request_claim_proof".into()),
            confidence: Some(0.74),
            rationale: Some("Official X proof missing".into()),
            supporting_evidence_json: serde_json::json!([{ "source": "website", "detail": "No backlink to X" }]),
            dissent_json: serde_json::json!([]),
            missing_proofs_json: serde_json::json!(["site backlink to x.com handle"]),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_value(&row).expect("serialize review entry");
        assert_eq!(json["role"], "critic");
        assert_eq!(json["recommended_action"], "request_claim_proof");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr src::models::review -- --nocapture`

Expected: FAIL with missing `src/models/review.rs` or unknown types such as `ToolOfficialLink` and `ReviewEntry`.

- [ ] **Step 3: Add the migration for official links, review runs, entries, and verdicts**

Create `migrations/019_agent_review_harness.sql` with concrete constraints and indexes.

```sql
CREATE TABLE tool_official_links (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  link_type TEXT NOT NULL CHECK (link_type IN ('github', 'website', 'x')),
  url TEXT NOT NULL,
  display_label TEXT NOT NULL,
  verification_status TEXT NOT NULL DEFAULT 'candidate'
    CHECK (verification_status IN ('candidate', 'claimed', 'verified', 'rejected')),
  official_badge_allowed BOOLEAN NOT NULL DEFAULT false,
  evidence_strength TEXT NOT NULL DEFAULT 'weak'
    CHECK (evidence_strength IN ('weak', 'medium', 'strong')),
  verification_method TEXT,
  discovered_from TEXT,
  verified_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  verified_at TIMESTAMPTZ,
  notes TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_tool_official_links_tool_id ON tool_official_links(tool_id);
CREATE INDEX idx_tool_official_links_status ON tool_official_links(verification_status);

CREATE TABLE review_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  queue TEXT,
  runner_name TEXT NOT NULL,
  prompt_version TEXT,
  snapshot_version TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed', 'discarded')),
  summary TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  created_by UUID REFERENCES profiles(id) ON DELETE SET NULL
);

CREATE TABLE review_entries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  review_run_id UUID NOT NULL REFERENCES review_runs(id) ON DELETE CASCADE,
  entry_type TEXT NOT NULL
    CHECK (entry_type IN ('agent_review', 'operator_note', 'system_event')),
  role TEXT NOT NULL,
  agent_label TEXT,
  recommended_action TEXT,
  confidence REAL,
  rationale TEXT,
  supporting_evidence_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  dissent_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  missing_proofs_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE operator_verdicts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  review_run_id UUID REFERENCES review_runs(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  from_status TEXT,
  to_status TEXT,
  from_claim_state TEXT,
  to_claim_state TEXT,
  reason_codes TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
  note TEXT,
  operator_id UUID NOT NULL REFERENCES profiles(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_review_runs_tool_id ON review_runs(tool_id, started_at DESC);
CREATE INDEX idx_review_entries_run_id ON review_entries(review_run_id, created_at ASC);
CREATE INDEX idx_operator_verdicts_tool_id ON operator_verdicts(tool_id, created_at DESC);
```

- [ ] **Step 4: Add Rust models and exports**

Create `src/models/review.rs`, then export it from `src/models/mod.rs`. Keep JSON fields as `serde_json::Value` so agents can attach structured packets without schema churn.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ToolOfficialLink {
    pub id: uuid::Uuid,
    pub tool_id: uuid::Uuid,
    pub link_type: String,
    pub url: String,
    pub display_label: String,
    pub verification_status: String,
    pub official_badge_allowed: bool,
    pub evidence_strength: String,
    pub verification_method: Option<String>,
    pub discovered_from: Option<String>,
    pub verified_by: Option<uuid::Uuid>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ReviewRun {
    pub id: uuid::Uuid,
    pub tool_id: uuid::Uuid,
    pub queue: Option<String>,
    pub runner_name: String,
    pub prompt_version: Option<String>,
    pub snapshot_version: String,
    pub status: String,
    pub summary: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<uuid::Uuid>,
}
```

Also update module declarations:

```rust
// src/models/mod.rs
pub mod review;
pub use review::{OperatorVerdict, ReviewEntry, ReviewRun, ToolOfficialLink};

// src/lib.rs
pub mod trust_verification;
```

- [ ] **Step 5: Run tests and migration prep**

Run:

- `cargo test --features ssr review_entry_preserves_role_and_action -- --nocapture`
- `sqlx migrate run`
- `cargo sqlx prepare`

Expected:

- unit tests PASS
- migration applies cleanly
- sqlx query metadata refresh completes without missing-table errors

- [ ] **Step 6: Commit**

```bash
git add migrations/019_agent_review_harness.sql src/models/review.rs src/models/mod.rs src/lib.rs .sqlx
git commit -m "feat: add agent review harness schema and models"
```

### Task 2: Build Trust Verification Helpers And Persistence APIs

**Files:**
- Create: `src/trust_verification.rs`
- Modify: `src/models/tool.rs`
- Modify: `src/server/functions.rs`
- Test: `src/trust_verification.rs`
- Test: `src/server/functions.rs`

- [ ] **Step 1: Write the failing trust-verification tests**

Describe the first-pass trust facts before implementing the helper module.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn demo_tool() -> crate::models::Tool {
        let review = crate::models::tool::default_review_fields();
        crate::models::Tool {
            id: uuid::Uuid::nil(),
            name: "BOB Gateway CLI".into(),
            slug: "bob-gateway-cli".into(),
            description: Some("Bridge CLI for Bitcoin and EVM chains".into()),
            function: "bridge".into(),
            asset_class: "crypto".into(),
            actor: "human".into(),
            tool_type: "cli".into(),
            repo_url: Some("https://github.com/bob-collective/bob".into()),
            homepage: Some("https://gobob.xyz".into()),
            npm_package: Some("@gobob/gateway-cli".into()),
            install_command: Some("npx @gobob/gateway-cli".into()),
            mcp_endpoint: None,
            chains: vec!["bitcoin".into(), "base".into()],
            status: "community".into(),
            official_team: None,
            trust_score: 0,
            approval_status: "approved".into(),
            submitted_by: None,
            rejection_reason: None,
            crypto_relevance_score: 82,
            crypto_relevance_reasons: vec!["npm scope matches github org".into()],
            relevance_status: "accepted".into(),
            install_risk_level: "low".into(),
            install_risk_reasons: vec!["npx install command".into()],
            requires_secret: false,
            safe_copy_command: Some("npx @gobob/gateway-cli".into()),
            quarantined_at: None,
            last_reviewed_at: None,
            review_policy_version: review.review_policy_version,
            claim_state: "unclaimed".into(),
            license: Some("MIT".into()),
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
            stars: 120,
            last_commit_at: Some(chrono::Utc::now() - chrono::TimeDelta::days(2)),
            source: "npm".into(),
            source_url: Some("https://www.npmjs.com/package/@gobob/gateway-cli".into()),
            logo_url: None,
            logo_monogram: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn trust_verification_promotes_explainable_facts() {
        let tool = demo_tool();

        let result = verify_tool_trust(&tool, &[]);

        assert!(result.total_score >= 70);
        assert!(result
            .trust_facts
            .iter()
            .any(|fact| fact.label == "Recent activity"));
        assert!(result
            .trust_facts
            .iter()
            .any(|fact| fact.label == "Domain and org aligned"));
    }

    #[test]
    fn trust_verification_hard_vetoes_curl_bash() {
        let mut tool = demo_tool();
        tool.install_command = Some("curl https://bad.sh | bash".into());

        let result = verify_tool_trust(&tool, &[]);

        assert_eq!(result.suggested_action, "quarantine");
        assert!(result.evidence_gaps.iter().any(|gap| gap.contains("unsafe install")));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr trust_verification -- --nocapture`

Expected: FAIL with missing `verify_tool_trust` and missing trust result types.

- [ ] **Step 3: Implement `src/trust_verification.rs`**

Return sub-scores, trust facts, evidence gaps, and suggested actions instead of a public truth score only.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TrustFact {
    pub label: String,
    pub detail: String,
    pub severity: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TrustVerificationResult {
    pub total_score: i32,
    pub identity_score: i32,
    pub operational_score: i32,
    pub install_safety_score: i32,
    pub claim_strength_score: i32,
    pub social_presence_score: i32,
    pub trust_facts: Vec<TrustFact>,
    pub evidence_gaps: Vec<String>,
    pub suggested_action: String,
}

pub fn verify_tool_trust(
    tool: &crate::models::Tool,
    official_links: &[crate::models::ToolOfficialLink],
) -> TrustVerificationResult {
    if tool
        .install_command
        .as_deref()
        .is_some_and(|cmd| cmd.contains("curl ") && cmd.contains("| bash"))
    {
        return TrustVerificationResult {
            total_score: 0,
            identity_score: 0,
            operational_score: 0,
            install_safety_score: 0,
            claim_strength_score: 0,
            social_presence_score: 0,
            trust_facts: vec![],
            evidence_gaps: vec!["unsafe install requires quarantine".into()],
            suggested_action: "quarantine".into(),
        };
    }

    let mut identity_score = 0;
    let mut operational_score = 0;
    let mut install_safety_score = 40;
    let mut claim_strength_score = 0;
    let mut social_presence_score = 0;
    let mut trust_facts = Vec::new();
    let mut evidence_gaps = Vec::new();

    if tool.repo_url.as_deref().is_some_and(|url| url.contains("github.com"))
        && tool.homepage.as_deref().is_some_and(|url| url.contains("https://"))
        && tool.npm_package.as_deref().is_some_and(|pkg| pkg.starts_with('@'))
    {
        identity_score += 30;
        trust_facts.push(TrustFact {
            label: "Domain and org aligned".into(),
            detail: "Repo, homepage, and package namespace form a consistent identity cluster".into(),
            severity: "positive".into(),
        });
    } else {
        evidence_gaps.push("identity alignment needs operator review".into());
    }

    if tool.last_commit_at.is_some_and(|at| at > chrono::Utc::now() - chrono::TimeDelta::days(7)) {
        operational_score += 20;
        trust_facts.push(TrustFact {
            label: "Recent activity".into(),
            detail: "Maintainer activity seen in the last 7 days".into(),
            severity: "positive".into(),
        });
    } else {
        evidence_gaps.push("recent maintainer activity not confirmed".into());
    }

    if tool.claim_state == "claimed" {
        claim_strength_score += 20;
        trust_facts.push(TrustFact {
            label: "Claimed by team".into(),
            detail: "Maintainer claim has been approved by operators".into(),
            severity: "positive".into(),
        });
    }

    if official_links.iter().any(|link| link.link_type == "x" && link.verification_status == "verified") {
        social_presence_score += 10;
    } else {
        evidence_gaps.push("official X proof missing".into());
    }

    trust_facts.push(TrustFact {
        label: "Verified install command".into(),
        detail: "Install command passed deterministic safety checks".into(),
        severity: "positive".into(),
    });

    let total_score = identity_score
        + operational_score
        + install_safety_score
        + claim_strength_score
        + social_presence_score;

    let suggested_action = if total_score >= 75 {
        "approve_community"
    } else {
        "needs_manual_research"
    };

    TrustVerificationResult {
        total_score,
        identity_score,
        operational_score,
        install_safety_score,
        claim_strength_score,
        social_presence_score,
        trust_facts,
        evidence_gaps,
        suggested_action: suggested_action.into(),
    }
}
```

- [ ] **Step 4: Add persistence and read APIs in `src/server/functions.rs`**

Create minimal CRUD helpers for:

- listing official links per tool
- writing review runs
- appending review entries
- writing operator verdicts
- loading trust facts for tool detail

Suggested server-function row shapes:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolTrustView {
    pub tool: Tool,
    pub official_links: Vec<ToolOfficialLink>,
    pub trust_facts: Vec<crate::trust_verification::TrustFact>,
}

#[server(GetToolTrustView, "/api")]
pub async fn get_tool_trust_view(slug: String) -> Result<ToolTrustView, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database pool not available"))?;

    let tool = sqlx::query_as::<_, Tool>(&format!(
        "SELECT * FROM tools WHERE slug = $1 AND {TOOLS_APPROVED_WHERE}"
    ))
    .bind(slug.trim())
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load tool trust view: {e}")))?;

    let official_links = sqlx::query_as::<_, ToolOfficialLink>(
        "SELECT * FROM tool_official_links WHERE tool_id = $1 ORDER BY link_type, created_at"
    )
    .bind(tool.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to load official links: {e}")))?;

    let trust = crate::trust_verification::verify_tool_trust(&tool, &official_links);

    Ok(ToolTrustView {
        tool,
        official_links,
        trust_facts: trust.trust_facts,
    })
}
```

- [ ] **Step 5: Run the targeted tests**

Run:

- `cargo test --features ssr trust_verification_promotes_explainable_facts -- --nocapture`
- `cargo test --features ssr trust_verification_hard_vetoes_curl_bash -- --nocapture`
- `cargo test --features ssr request_tool_claim -- --nocapture`

Expected:

- trust tests PASS
- existing claim tests still PASS

- [ ] **Step 6: Commit**

```bash
git add src/trust_verification.rs src/models/tool.rs src/server/functions.rs
git commit -m "feat: add trust verification helpers and persistence APIs"
```

### Task 3: Extend The Operator Harness And Review Logging Pipeline

**Files:**
- Modify: `src/server/operator_harness.rs`
- Modify: `src/server/functions.rs`
- Modify: `src/models/review.rs`
- Test: `src/server/operator_harness.rs`
- Test: `tests/agent_review_harness_flow.rs`

- [ ] **Step 1: Write the failing harness-flow test**

Define the behavior where an external coding agent run becomes a persisted review timeline.

```rust
#[tokio::test]
async fn harness_run_persists_review_entries_and_requires_human_verdict() {
    let database_url = std::env::var("SUPABASE_URL_TEST").expect("SUPABASE_URL_TEST must be set");
    let pool = sqlx::PgPool::connect(&database_url).await.expect("connect test db");

    let tool_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO tools (
            name, slug, description, function, asset_class, actor, type,
            repo_url, homepage, npm_package, install_command, chains,
            status, trust_score, approval_status, source, source_url
        )
        VALUES (
            'BOB Gateway CLI', 'bob-gateway-cli-review-test', 'Bridge CLI', 'bridge', 'crypto',
            'human', 'cli', 'https://github.com/bob-collective/bob', 'https://gobob.xyz',
            '@gobob/gateway-cli', 'npx @gobob/gateway-cli', ARRAY['bitcoin', 'base'],
            'community', 0, 'approved', 'npm', 'https://www.npmjs.com/package/@gobob/gateway-cli'
        )
        RETURNING id
        "#,
    )
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
            supporting_evidence_json: serde_json::json!([{ "source": "github", "detail": "repo/homepage aligned" }]),
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
    assert_eq!(timeline[0].recommended_action.as_deref(), Some("request_claim_proof"));
    assert!(timeline[0].confidence.unwrap() > 0.7);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr harness_run_persists_review_entries_and_requires_human_verdict -- --nocapture`

Expected: FAIL with missing create/append/load helpers.

- [ ] **Step 3: Add harness write paths and timeline loading**

Extend `src/server/operator_harness.rs` so it remains bounded/read-only for recommendations, but can also persist external-agent review sessions via explicit admin-gated write endpoints. Implement the shared helpers as `insert_review_run`, `insert_review_entry`, and `list_review_entries`.

```rust
#[derive(Debug, Deserialize)]
pub struct CreateReviewRunInput {
    pub tool_id: uuid::Uuid,
    pub queue: Option<String>,
    pub runner_name: String,
    pub prompt_version: Option<String>,
    pub snapshot_version: String,
}

#[derive(Debug, Deserialize)]
pub struct AppendReviewEntryInput {
    pub review_run_id: uuid::Uuid,
    pub role: String,
    pub agent_label: Option<String>,
    pub recommended_action: Option<String>,
    pub confidence: Option<f32>,
    pub rationale: Option<String>,
    pub supporting_evidence_json: serde_json::Value,
    pub dissent_json: serde_json::Value,
    pub missing_proofs_json: serde_json::Value,
}
```

Admin-only write endpoints should call shared helpers in `src/server/functions.rs`, not duplicate SQL.

- [ ] **Step 4: Add operator verdict write helper**

Create a shared helper that updates the tool row and appends an immutable verdict record in one transaction.

```rust
pub async fn write_operator_verdict(
    pool: &sqlx::PgPool,
    operator_id: uuid::Uuid,
    input: WriteOperatorVerdictInput,
) -> Result<OperatorVerdict, ServerFnError> {
    let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(format!("transaction failed: {e}")))?;

    let tool = sqlx::query_as::<_, Tool>("SELECT * FROM tools WHERE id = $1 FOR UPDATE")
        .bind(input.tool_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("failed to lock tool: {e}")))?;

    // Update tool status / claim_state / review timestamps here.

    let verdict = sqlx::query_as::<_, OperatorVerdict>(
        r#"
        INSERT INTO operator_verdicts (
            tool_id, review_run_id, action, from_status, to_status,
            from_claim_state, to_claim_state, reason_codes, note, operator_id
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        RETURNING *
        "#,
    )
    .bind(input.tool_id)
    .bind(input.review_run_id)
    .bind(input.action)
    .bind(tool.status.clone())
    .bind(input.to_status.clone())
    .bind(tool.claim_state.clone())
    .bind(input.to_claim_state.clone())
    .bind(input.reason_codes)
    .bind(input.note)
    .bind(operator_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("failed to insert operator verdict: {e}")))?;

    tx.commit().await.map_err(|e| ServerFnError::new(format!("commit failed: {e}")))?;
    Ok(verdict)
}
```

- [ ] **Step 5: Run harness tests**

Run:

- `cargo test --features ssr operator_harness -- --nocapture`
- `cargo test --features ssr --test agent_review_harness_flow -- --nocapture`

Expected:

- harness unit tests PASS
- end-to-end flow test PASS with persisted review run, entry, and verdict records

- [ ] **Step 6: Commit**

```bash
git add src/server/operator_harness.rs src/server/functions.rs tests/agent_review_harness_flow.rs
git commit -m "feat: persist external agent review runs and verdicts"
```

### Task 4: Redesign `/admin/tools` Into The Review Workbench

**Files:**
- Create: `src/components/admin_review_timeline.rs`
- Create: `src/components/admin_review_decision_panel.rs`
- Modify: `src/pages/admin/tools.rs`
- Modify: `src/server/functions.rs`
- Test: `src/pages/admin/tools.rs`

- [ ] **Step 1: Write the failing UI-state test**

Describe the new focus-mode workbench state helpers before changing the page markup.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_tool_prefers_first_queue_item_when_query_missing() {
        let ids = vec!["bob-gateway-cli".to_string(), "zapper-mcp".to_string()];
        let selected = derive_selected_slug(None, &ids);
        assert_eq!(selected.as_deref(), Some("bob-gateway-cli"));
    }

    #[test]
    fn summary_cards_include_claim_pending_bucket() {
        let cards = build_summary_cards(24, 5, 12, 4);
        assert!(cards.iter().any(|card| card.label == "Claim Pending"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr selected_tool_prefers_first_queue_item_when_query_missing -- --nocapture`

Expected: FAIL with missing helper functions.

- [ ] **Step 3: Create focused workbench components**

The admin page should stop rendering one independent card per tool and instead render:

- queue rail
- top summary rail
- selected-tool review timeline
- sticky decision panel

Example component interface:

```rust
#[component]
pub fn AdminReviewTimeline(
    entries: Vec<crate::models::ReviewEntry>,
    operator_verdicts: Vec<crate::models::OperatorVerdict>,
) -> impl IntoView {
    view! {
        <section class="rounded-xl border border-[#E5E5E5] bg-white p-5">
            <h2 class="text-[16px] font-semibold mb-4">"Review timeline"</h2>
            <div class="space-y-4">
                {entries.into_iter().map(|entry| view! {
                    <article class="border-l-2 border-[#E5E5E5] pl-4">
                        <p class="text-[12px] text-[#6B6B6B]">
                            {entry.role.clone()} " · " {entry.agent_label.clone().unwrap_or_else(|| "system".into())}
                        </p>
                        <p class="text-[14px] mt-1">{entry.rationale.clone().unwrap_or_else(|| "No rationale".into())}</p>
                    </article>
                }).collect_view()}
            </div>
        </section>
    }
}
```

- [ ] **Step 4: Rework `src/pages/admin/tools.rs` around one selected candidate**

Replace the list of repeated `ReviewToolRow` cards with:

```rust
let selected_slug = Memo::new(move |_| derive_selected_slug(
    query.get().get("selected").map(|s| s.to_string()),
    &queue_items.get().unwrap_or_default()
        .into_iter()
        .map(|item| item.tool.slug)
        .collect::<Vec<_>>(),
));
```

Then render:

```rust
<div class="grid grid-cols-1 xl:grid-cols-[220px_minmax(0,1fr)_320px] gap-4">
    <QueueRail ... />
    <AdminReviewTimeline entries=timeline_entries verdicts=verdicts />
    <AdminReviewDecisionPanel tool=selected_tool trust=trust_panel links=official_links />
</div>
```

- [ ] **Step 5: Run focused verification**

Run:

- `cargo fmt --check`
- `cargo build --features ssr`

Expected:

- formatting PASS
- SSR build PASS with no missing component imports

- [ ] **Step 6: Commit**

```bash
git add src/components/admin_review_timeline.rs src/components/admin_review_decision_panel.rs src/pages/admin/tools.rs src/server/functions.rs
git commit -m "feat: redesign admin tools into review workbench"
```

### Task 5: Add Public Trust Facts And Official Link UI

**Files:**
- Create: `src/components/tool_trust_facts.rs`
- Create: `src/components/official_links_list.rs`
- Modify: `src/components/tool_detail_content.rs`
- Modify: `src/components/tool_card.rs`
- Modify: `src/pages/tool_detail.rs`
- Modify: `src/server/functions.rs`
- Test: `src/trust_verification.rs`

- [ ] **Step 1: Write the failing public-trust tests**

Capture the rule that public UI shows facts, not raw trust scores.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_trust_facts_hide_raw_numeric_score() {
        let facts = vec![
            crate::trust_verification::TrustFact {
                label: "Claimed by team".into(),
                detail: "Maintainer claim approved by operators".into(),
                severity: "positive".into(),
            }
        ];

        let html = trust_fact_summary_text(&facts);

        assert!(html.contains("Claimed by team"));
        assert!(!html.contains("81"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr public_trust_facts_hide_raw_numeric_score -- --nocapture`

Expected: FAIL with missing component render helper.

- [ ] **Step 3: Create reusable trust-facts and official-links components**

Example interfaces:

```rust
#[component]
pub fn ToolTrustFacts(
    facts: Vec<crate::trust_verification::TrustFact>,
) -> impl IntoView {
    view! {
        <section class="rounded-xl border border-[#E5E5E5] bg-[#FAFAFA] p-4">
            <h3 class="text-[14px] font-semibold mb-3">"Why this looks trustworthy"</h3>
            <ul class="space-y-2">
                {facts.into_iter().map(|fact| view! {
                    <li class="text-[14px]">
                        <span class="font-medium">{fact.label.clone()}</span>
                        <span class="text-[#6B6B6B]">" — " {fact.detail.clone()}</span>
                    </li>
                }).collect_view()}
            </ul>
        </section>
    }
}

pub fn trust_fact_summary_text(facts: &[crate::trust_verification::TrustFact]) -> String {
    facts
        .iter()
        .map(|fact| format!("{} — {}", fact.label, fact.detail))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[component]
pub fn OfficialLinksList(
    links: Vec<crate::models::ToolOfficialLink>,
) -> impl IntoView {
    view! {
        <section class="rounded-xl border border-[#E5E5E5] bg-white p-4">
            <h3 class="text-[14px] font-semibold mb-3">"Official links"</h3>
            // Render GitHub / Website / X labels here with neutral fallback copy.
        </section>
    }
}
```

- [ ] **Step 4: Wire the detail page and light card markers**

In `src/components/tool_detail_content.rs`, insert trust facts and official links below description or install, not above the title.

```rust
<ToolTrustFacts facts=trust_facts.clone() />
<OfficialLinksList links=official_links.clone() />
```

For `src/components/tool_card.rs`, add only lightweight markers:

```rust
{if tool.claim_state == "claimed" {
    view! { <span class="badge badge-neutral">"Claimed by team"</span> }.into_any()
} else {
    ().into_any()
}}
```

- [ ] **Step 5: Run UI verification**

Run:

- `cargo build --features ssr`
- `./scripts/smoke-test.sh http://localhost:3000`
- `node scripts/browser-smoke.mjs http://localhost:3000`
- `node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots`

Expected:

- build PASS
- smoke PASS
- browser smoke PASS
- desktop and mobile screenshots show readable trust facts with no horizontal overflow

- [ ] **Step 6: Commit**

```bash
git add src/components/tool_trust_facts.rs src/components/official_links_list.rs src/components/tool_detail_content.rs src/components/tool_card.rs src/pages/tool_detail.rs src/server/functions.rs
git commit -m "feat: add public trust facts and official link UI"
```

### Task 6: Upgrade Submit And Claim UX For Proof Collection

**Files:**
- Create: `src/components/claim_status_timeline.rs`
- Modify: `src/pages/submit.rs`
- Modify: `src/models/submission.rs`
- Modify: `src/server/functions.rs`
- Test: `src/server/functions.rs`

- [ ] **Step 1: Write the failing claim-validation tests**

Add validation tests for the stronger claim path.

```rust
#[test]
fn validate_claim_tool_input_requires_verification_note() {
    let input = ClaimToolInput {
        slug: "bob-gateway-cli".into(),
        contact_email: Some("team@gobob.xyz".into()),
        verification_note: "".into(),
    };

    let err = validate_claim_tool_input(&input).expect_err("empty verification note should fail");
    assert!(err.contains("verification note"));
}

#[test]
fn validate_claim_proof_urls_reject_non_http_links() {
    let urls = vec!["javascript:alert(1)".to_string()];
    let err = validate_claim_proof_urls(&urls).expect_err("unsafe links should fail");
    assert!(err.contains("http"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ssr validate_claim_tool_input_requires_verification_note -- --nocapture`

Expected: FAIL with missing stronger validation helper.

- [ ] **Step 3: Add claim-proof fields and UI split**

Keep one page but split it into:

- `Suggest a tool`
- `Claim this tool`

Add explicit proof-oriented fields when `official_team_claim` is checked or when the user enters the claim branch.

```rust
let claim_mode = RwSignal::new(false);

// When claim mode:
// - official team name
// - official email
// - github url
// - website url
// - x url
// - proof note
// - optional proof links
```

Render a status helper:

```rust
<ClaimStatusTimeline
    steps=vec![
        "Submitted".into(),
        "Under review".into(),
        "Needs more proof".into(),
        "Claim approved".into(),
    ]
/>
```

- [ ] **Step 4: Update claim persistence**

Extend the claim path in `src/server/functions.rs` to save richer proof context and pre-seed `tool_official_links` rows as `candidate` when safe URLs are supplied.

```rust
if let Some(url) = input.github_url.as_deref() {
    insert_candidate_official_link(&mut tx, tool.id, "github", url, "Claimed GitHub").await?;
}
```

- [ ] **Step 5: Run targeted tests**

Run:

- `cargo test --features ssr validate_claim_tool_input_requires_verification_note -- --nocapture`
- `cargo test --features ssr request_tool_claim -- --nocapture`

Expected:

- stronger validation PASS
- existing claim path behavior PASS with richer stored proof

- [ ] **Step 6: Commit**

```bash
git add src/components/claim_status_timeline.rs src/pages/submit.rs src/models/submission.rs src/server/functions.rs
git commit -m "feat: expand submit and claim proof UX"
```

### Task 7: Final Verification, SQLx Refresh, And Handoff Packet

**Files:**
- Modify: `docs/superpowers/plans/2026-06-28-agent-review-harness-trust-ui-implementation-plan.md`
- Test: `tests/agent_review_harness_flow.rs`

- [ ] **Step 1: Run the full quality gate**

Run:

```bash
./scripts/disk-guard.sh
cargo test --features ssr
cargo clippy --features ssr -- -W clippy::all
cargo fmt --check
./scripts/release-build.sh
./scripts/verify-bundle.sh
./scripts/restart-dev.sh
./scripts/smoke-test.sh http://localhost:3000
node scripts/browser-smoke.mjs http://localhost:3000
node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots
```

Expected:

- test suite PASS
- clippy PASS with no new warnings promoted to errors
- format PASS
- release bundle verification PASS
- restart and smoke PASS
- browser smoke PASS
- screenshots confirm readable desktop/mobile layouts

- [ ] **Step 2: Capture rollout notes for external coding agents**

Append a short execution note to the plan or a sibling progress file that records:

- migration applied
- queue workbench route verified
- detail trust UI verified
- claim flow verified
- remaining polish issues if any

Use this note shape:

```md
## Rollout Notes

- Migration `019_agent_review_harness.sql` applied locally.
- `/admin/tools` shows queue rail, review timeline, and sticky decision panel.
- `/tools/:slug` shows trust facts and official links without exposing raw trust score.
- `/submit` supports claim-proof capture and status timeline.
- Remaining follow-up: tune copy and spacing after live operator use.
```

- [ ] **Step 3: Commit**

```bash
git add .sqlx docs/superpowers/plans/2026-06-28-agent-review-harness-trust-ui-implementation-plan.md
git commit -m "chore: verify agent review harness trust UI rollout"
```
