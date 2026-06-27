# External Agent Goal Packet

Use this when handing the full implementation to Grok or any other coding agent as a single goal.

---

## Short Goal

Implement the full-stack v1 of OnchainAI's agent review harness and trust UI in one integrated pass.

This includes:

- operator review workbench on `/admin/tools`
- public trust facts and official links on `/tools/:slug`
- upgraded submit/claim proof flow on `/submit`
- new persistence for official links, review runs, review entries, and operator verdicts
- trust verification helpers
- operator harness extensions for model-agnostic external agent review logging

Do not redesign the product away from the approved architecture. Build on the existing Rust/Leptos/Axum/sqlx codebase and follow repo rules exactly.

---

## Read First

Read these before touching code:

1. `AGENTS.md`
2. `DESIGN.md`
3. `docs/UI_UX_DESIGN.md`
4. `docs/BUILD_DEPLOY_RULES.md`
5. `docs/superpowers/specs/2026-06-28-agent-review-harness-and-trust-ui-design.md`
6. `docs/superpowers/plans/2026-06-28-agent-review-harness-trust-ui-implementation-plan.md`

---

## Product Direction

You are implementing a **fact-first review system**, not an autonomous approval engine.

Key product rules:

- external AI/coding agents are research assistants, not approvers
- human operator remains final authority
- raw numeric trust score is operator-facing only
- public UI must show explainable trust facts, not opaque AI scores
- `Official` labels require strong proof and human approval
- `Featured` is editorial, not a trust or safety claim

---

## Required Deliverables

### 1. Operator Review Workbench

Implement a readable review workbench on `/admin/tools` with:

- top promotion summary rail
- left queue rail
- center review timeline
- right sticky decision panel

The center timeline should be the visual focus.

The decision panel must show:

- Official GitHub
- Official Website
- Official X
- evidence strength
- claimed/verified trust facts
- next required proof
- final actions

### 2. Public Trust UI

Extend tool detail and lightly extend tool cards so public users can understand why a tool looks trustworthy.

Public UI must:

- not expose raw trust score
- show trust facts such as `Claimed by team`, `Verified install command`, `Recent activity`
- show official links with icons
- fall back to neutral link labels when proof is weak

### 3. Submit / Claim Proof Flow

Upgrade `/submit` so it supports:

- regular tool suggestion
- stronger claim/proof flow
- official GitHub / website / X inputs
- proof note and proof links
- readable claim status timeline

### 4. Data Model And Harness

Implement:

- `tool_official_links`
- `review_runs`
- `review_entries`
- `operator_verdicts`
- `trust_verification.rs`

Extend the current operator harness so external coding agents can log:

- review runs
- review timeline entries
- recommendations
- dissent
- missing proof requests

---

## Hard Constraints

- Keep the stack Rust single-binary: Leptos SSR + Axum + sqlx.
- Follow current UI language rules: public UI text in English.
- No emoji in UI text.
- Keep the calm light theme and existing design system direction.
- Do not turn the UI into a dense crypto dashboard.
- Optimize for readability first.
- Do not expose secrets or operator-only payout data to public clients.
- Do not introduce automatic official promotion.
- Do not expose AI vote count or AI confidence publicly.
- Do not rebuild architecture around Grok-only features; keep the harness model-agnostic.

---

## Data And Status Rules

Keep these semantics:

- public listing status: `community -> verified -> official`
- claim state: `unclaimed -> claim_pending -> claimed -> disputed/revoked`
- official links: `candidate -> claimed -> verified -> rejected`

Important:

- `Official GitHub`, `Official Website`, `Official X` only when sufficiently verified
- otherwise show neutral labels like `GitHub`, `Website`, `X profile`
- `community` approval still goes through human operator action in v1

---

## UI Recommendation

Prioritize this structure for `/admin/tools`:

- top summary rail
- left queue rail
- center review timeline
- right sticky decision panel

If you need to choose between density and readability, choose readability.

The operator should mostly review **one selected candidate at a time**.

---

## Suggested Internal Parallelization

If you want to use many subagents internally, split by concern:

- Worker A: migrations + models + SQLx
- Worker B: trust verification + server functions
- Worker C: operator harness + review logging
- Worker D: `/admin/tools` review workbench UI
- Worker E: public trust UI
- Worker F: `/submit` claim/proof UX
- Worker G: test + smoke + visual verification

But merge back into one coherent branch and one integrated result.

---

## Acceptance Criteria

The work is complete only when all of the following are true:

1. `/admin/tools` behaves like a review workbench rather than a flat list of independent cards.
2. Review timeline data is persisted and rendered.
3. Operator verdicts are persisted and tied to tool review state changes.
4. Official links are stored separately and verified independently.
5. `/tools/:slug` shows trust facts and official links without exposing raw trust score.
6. `/submit` supports proof-oriented claim flow.
7. The operator harness supports model-agnostic external review logging.
8. Tests pass.
9. Clippy passes.
10. Release build, bundle verification, smoke checks, browser smoke, and visual snapshots all pass.

---

## Required Verification

Run these before claiming completion:

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

If Playwright is missing, state that clearly and stop short of claiming browser verification passed.

---

## Output Format

When you report back:

1. Start with the implemented outcome across the 3 surfaces.
2. List any schema changes and new tables.
3. List verification commands actually run and whether they passed.
4. Call out remaining risks or follow-up polish items.

Do not give a vague “done” without verification evidence.
