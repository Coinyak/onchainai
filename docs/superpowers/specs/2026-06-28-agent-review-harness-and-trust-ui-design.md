# Agent Review Harness And Trust UI Design

> Related docs: [[../../MVP_DESIGN]] | [[../../UI_UX_DESIGN]] | [[../../../DESIGN]] | [[../../BUILD_DEPLOY_RULES]] | [[../../../AGENTS.md]]
>
> Date: 2026-06-28
> Status: Approved design for implementation planning
> Scope: Full-stack operator review workbench, public trust UI, submit/claim flow, trust verification model, and external agent review harness

---

## 0. One-line Direction

OnchainAI should not treat AI as an auto-approval engine. It should act as a **fact-first review system** where crawlers discover tools, trust verification structures evidence, external coding agents produce review packets, and the operator makes the final decision that becomes the canonical verdict.

---

## 1. Why This Design

### 1.1 Problem

Discovery alone is not enough for a crypto tools directory.

The hard problem is not just finding tools, but deciding:

- whether a project is actually crypto-relevant
- whether its install path is safe
- whether GitHub / website / X accounts are truly official
- whether a listing should remain community, become verified, become official, or get quarantined

The current system already has useful building blocks:

- crawler ingestion
- relevance scoring
- install-safety scoring
- review queues
- claim requests
- featured cards
- a bounded operator harness skeleton

What is missing is a coherent product model that ties them together across:

1. operator review
2. public trust presentation
3. claim and proof collection
4. external AI-assisted review logging

### 1.2 Competitive Pattern Summary

The research pattern across registry and directory products is consistent:

- Official registries tend to be conservative and focus on ownership verification.
- Submission/claim flows are common even when discovery is automated.
- Strong products separate discovery, verification, and editorial promotion.
- Public trust is usually expressed through evidence and labels, not a raw opaque score.
- High-trust states such as `official` still retain human approval.

This design follows that pattern while giving OnchainAI a stronger discovery and agent-assisted review workflow than current peers.

### 1.3 Product Positioning

OnchainAI becomes:

- a **discovery engine** for fragmented crypto tools
- a **trust review workbench** for operators
- a **public trust surface** for users and agents
- a **bounded evidence provider** for external coding agents

It does **not** become:

- a fully autonomous approval machine
- a model-specific Grok feature
- a public AI-score marketplace

---

## 2. Design Principles

### 2.1 Human Final Authority

AI may recommend actions. Only the operator can:

- approve public community listings
- mark verified
- mark official
- quarantine or unquarantine
- send a tool into featured editorial flow

### 2.2 Public Trust Must Be Explainable

Public users should not see a naked `trust_score=81` and be expected to infer meaning.

Public trust should be shown with evidence-backed labels such as:

- `Claimed by team`
- `Verified install command`
- `Active recently`
- `Domain and org aligned`
- `Official GitHub`
- `Official Website`
- `Official X`

Raw numeric trust remains operator-facing.

### 2.3 Official Means Strong Proof

`Official` must never be granted automatically from crawler signals or AI confidence alone.

Strong proof is required, such as:

- claim approval by operator
- official site backlink to the claimed GitHub or X profile
- repo-level or domain-level proof
- equivalent operator-reviewed ownership evidence

### 2.4 Readability Over Density

The operator console should optimize for accurate judgment, not maximal dashboard density.

The preferred interaction is:

- pick one candidate
- read the review timeline
- inspect official link proofs
- make one decision

### 2.5 Agent-Agnostic Review Harness

The system should support Grok, Codex, Claude Code, and other coding agents equally.

OnchainAI provides:

- bounded snapshot
- evidence packet
- storage for review runs
- storage for review entries
- operator verdict history

Agents provide:

- recommendations
- rationale
- dissent
- missing-proof requests

---

## 3. Information Architecture

### 3.1 Core Product Surfaces

This design deliberately connects three user-facing surfaces.

1. **Operator Review Workbench**
2. **Public Trust UI**
3. **Submit / Claim Flow**

These are connected by one review model, one proof model, and one verdict model.

### 3.2 Lifecycle Model

Public listing lifecycle:

`discovered -> community -> verified -> official`

Editorial lifecycle:

`not_featured -> featured_queue -> featured_live`

Claim lifecycle:

`unclaimed -> claim_pending -> claimed -> disputed/revoked`

Official-link lifecycle:

`candidate -> claimed -> verified -> rejected`

Review lifecycle:

`snapshot_created -> review_run_started -> review_entries_recorded -> operator_verdict_written`

---

## 4. Operator Review Workbench

### 4.1 Recommended Interaction Model

The operator console should be a **review workbench**, not a dashboard-first console.

Recommended layout:

- **Top summary rail**
  - `Discovered`
  - `Claim Pending`
  - `Verified Ready`
  - `Featured Queue`
- **Left queue rail**
  - `New candidates`
  - `Needs manual research`
  - `Claim pending`
  - `Reported`
  - `High risk`
- **Center review timeline**
  - widest column
  - chronological AI review entries
  - operator notes
  - missing-proof checkpoints
- **Right sticky decision panel**
  - official links
  - trust facts
  - evidence strength
  - final actions

### 4.2 Why This Structure

This combines the strongest parts of two models:

- **Inbox model** for rapid queue work
- **Promotion pipeline model** for lifecycle awareness

The queue rail makes the current system familiar.

The summary rail surfaces the larger lifecycle:

- what is newly discovered
- what is waiting for claim proof
- what is ready for verified promotion
- what is ready for featured editorial consideration

### 4.3 Readability Rules

The operator console must bias for readability:

- one selected tool at a time
- center timeline as the widest region
- raw evidence collapsed by default
- short evidence summaries before long blobs
- sticky decision panel on desktop
- mobile/tablet should degrade to focus mode, not cram three columns

### 4.4 Review Timeline Content

Each timeline entry should show:

- source agent name or operator name
- role
- timestamp
- recommended action
- confidence
- short rationale
- expandable evidence details
- dissent or uncertainty when present

Example roles:

- `identity`
- `operational`
- `install_safety`
- `claim_proof`
- `critic`
- `judge`
- `operator_note`

### 4.5 Decision Panel Content

The right-side decision panel should include:

- `Official GitHub`
- `Official Website`
- `Official X`
- evidence strength for each link
- `Claimed by team`
- `Verified install`
- `Recent activity`
- `Domain/org match`
- current lifecycle state
- next required proof

Actions:

- `Approve community`
- `Request claim proof`
- `Mark verified`
- `Mark official`
- `Quarantine`
- `Send to featured`

`Mark official` must remain disabled or gated until strong proof exists.

### 4.6 Existing Surfaces To Extend

Primary route to extend:

- `/admin/tools`

Supporting operator surfaces:

- `/admin/dashboard`
- `/admin/featured`
- harness endpoints under the current operator harness infrastructure

Relevant files:

- `src/pages/admin/tools.rs`
- `src/server/functions.rs`
- `src/server/operator_harness.rs`

---

## 5. Public Trust UI

### 5.1 Public Principle

Public trust UI should show **trust facts**, not AI internals.

Hide:

- raw AI confidence
- raw agent vote count
- opaque trust scores

Show:

- verified links
- verified facts
- recent activity
- official claim status

### 5.2 List Card Changes

Tool cards may show light trust markers such as:

- `Verified`
- `Official`
- `Claimed by team`
- `Verified install`

Do not overload cards with too many micro-signals.

Card-level trust should stay scannable.

### 5.3 Tool Detail Changes

The detail page becomes the public trust explanation surface.

Recommended additions:

- `Why this looks trustworthy`
- `Official links`
- `Trust notes`
- `Recent activity`
- `Install verification`

Recommended trust facts:

- `Claimed by team`
- `Verified install command`
- `Active in last 7 days`
- `Domain and org aligned`

### 5.4 Official Link Display Rules

Links should be treated independently.

Examples:

- `Official GitHub`
- `Official Website`
- `Official X`

If a link is not strongly verified:

- show neutral label such as `GitHub`, `Website`, or `X profile`
- do not apply official language

### 5.5 Iconography

Use lightweight icon treatment:

- GitHub mark
- globe icon for website
- X icon

This aligns with the user's request for clearly surfaced official GitHub, site, and X links.

### 5.6 Existing Surfaces To Extend

Primary route:

- `/tools/:slug`

Likely components:

- `src/pages/tool_detail.rs`
- `src/components/tool_detail_content.rs`
- `src/components/tool_card.rs`
- `src/components/tool_listing_actions.rs`

---

## 6. Submit / Claim Flow

### 6.1 Product Goal

Submission and claim should be related but clearly distinct.

- **Submission** says: “please review this tool”
- **Claim** says: “I represent this project and want official recognition”

### 6.2 Submission Path

The existing public submission flow remains the intake path for:

- new tool suggestions
- partial metadata
- community candidate additions

Minimal friction is acceptable because crypto relevance is gated at public approval, not at intake.

### 6.3 Claim Path

Claim is a stronger, proof-oriented flow.

Required fields should include:

- team or company name
- official email
- GitHub org or repo URL
- official website URL
- X profile URL
- freeform proof note

Optional but valuable proof artifacts:

- site backlink
- docs reference
- repo file proof
- other operator note

### 6.4 Claim Review UX

Claim submissions should surface to the operator console as a distinct queue.

Operators should be able to verify links individually:

- approve GitHub as official before X
- approve website before GitHub if site proof is strongest
- reject one link without rejecting the full tool

### 6.5 Claim Status Timeline

Public-facing or submitter-facing state should show:

- `Submitted`
- `Under review`
- `Needs more proof`
- `Claim approved`
- `Claim disputed`

### 6.6 Existing Surfaces To Extend

Primary route:

- `/submit`

Existing data model support:

- `tool_claim_requests`
- `tools.claim_state`

Likely files:

- `src/pages/submit.rs`
- `src/models/submission.rs`
- `src/server/functions.rs`

---

## 7. Trust Verification Model

### 7.1 Core Decision

`trust_score` may remain as an operator-facing summary signal, but it must not be the public source of truth.

The actual decision engine should produce:

- score breakdown
- evidence list
- evidence gaps
- recommended next action

### 7.2 Proposed Breakdown

`trust_verification.rs` should compute at least these dimensions:

- `identity`
  - npm scope ↔ GitHub org ↔ docs domain alignment
- `operational`
  - recent commits, recent releases, site health
- `install_safety`
  - install parser results, curl|bash checks, typosquat checks
- `claim_strength`
  - backlink proof, claim approval, operator-reviewed ownership
- `social_presence`
  - candidate X profile presence and linkage quality

### 7.3 Output Shape

The module should return a structured result, not only a number.

Suggested shape:

```rust
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
```

### 7.4 Public vs Operator Use

Operator surfaces use:

- total score
- sub-scores
- evidence gaps
- suggested action

Public surfaces use:

- trust facts only
- verified link states
- recent activity facts

### 7.5 Hard Stops

The verifier should always support hard veto or quarantine logic for:

- `curl | bash`
- obvious typosquat signals
- strong identity mismatch
- abandoned or deceptive clone patterns

---

## 8. External Agent Review Harness

### 8.1 Role Of External Agents

External coding agents are **research assistants**, not approvers.

They are used because the operator may want multiple viewpoints and has no practical summon limit.

### 8.2 OnchainAI Responsibilities

OnchainAI should provide:

- bounded snapshot generation
- redacted evidence packet
- storage for review runs
- storage for review entries
- storage for operator verdicts

### 8.3 Agent Responsibilities

Agents should return:

- recommended action
- confidence
- rationale
- supporting evidence summary
- dissent or uncertainty
- missing proof requests

### 8.4 Recommended Review Roles

The harness should support at least these role labels:

- `identity`
- `operational`
- `install_safety`
- `claim_proof`
- `critic`
- `judge`

### 8.5 Why This Beats “AI Looking At A Screen”

Agents should review the structured packet, not infer from UI screenshots.

Reason:

- UI is a summary layer
- evidence needs to be bounded and reproducible
- review quality is higher when agents inspect structured facts and excerpts

The UI is for the operator to validate agent output, not the primary input to agent reasoning.

### 8.6 Existing Harness Baseline

Current starting point:

- `src/server/operator_harness.rs`

This should be extended rather than replaced.

### 8.7 Suggested API Extensions

Likely additions:

- create review run
- append review entries
- fetch review timeline for a tool
- write operator verdict
- fetch historical verdict log

The protocol should remain model-agnostic and minimal.

---

## 9. Data Model Additions

### 9.1 Keep Existing Tables

Keep and extend:

- `tools`
- `tool_submissions`
- `tool_claim_requests`
- `featured_cards`

### 9.2 New Table: `tool_official_links`

Purpose:

- store candidate and verified official links per tool

Suggested columns:

- `id`
- `tool_id`
- `link_type` (`github | website | x`)
- `url`
- `display_label`
- `verification_status` (`candidate | claimed | verified | rejected`)
- `official_badge_allowed`
- `evidence_strength` (`weak | medium | strong`)
- `verification_method`
- `discovered_from`
- `verified_by`
- `verified_at`
- `notes`
- `created_at`
- `updated_at`

### 9.3 New Table: `review_runs`

Purpose:

- represent one AI-assisted review session

Suggested columns:

- `id`
- `tool_id`
- `queue`
- `runner_name`
- `prompt_version`
- `snapshot_version`
- `status`
- `summary`
- `started_at`
- `completed_at`
- `created_by`

### 9.4 New Table: `review_entries`

Purpose:

- store one agent review or operator note within a run

Suggested columns:

- `id`
- `review_run_id`
- `entry_type` (`agent_review | operator_note | system_event`)
- `role`
- `agent_label`
- `recommended_action`
- `confidence`
- `rationale`
- `supporting_evidence_json`
- `dissent_json`
- `missing_proofs_json`
- `created_at`

### 9.5 New Table: `operator_verdicts`

Purpose:

- append-only record of final human decisions

Suggested columns:

- `id`
- `tool_id`
- `review_run_id`
- `action`
- `from_status`
- `to_status`
- `from_claim_state`
- `to_claim_state`
- `reason_codes`
- `note`
- `operator_id`
- `created_at`

### 9.6 Why These Are Separate

Separation keeps the product understandable:

- links are not verdicts
- verdicts are not AI opinions
- AI opinions are not public truth

---

## 10. UI States And Labels

### 10.1 Operator Labels

Internal operator language may include:

- `candidate`
- `verified ready`
- `official proof missing`
- `claim pending`
- `high risk install`

### 10.2 Public Labels

Public language should stay calm and explicit:

- `Official GitHub`
- `Official Website`
- `Official X`
- `Claimed by team`
- `Verified install`
- `Active recently`
- `Trust note`

Avoid marketing-like badges and avoid unexplained scoring.

### 10.3 Featured Is Editorial, Not Trust

`Featured` must be treated as editorial promotion, not evidence of safety or authenticity.

It should never imply:

- official ownership
- install safety
- payment verification

---

## 11. Full-stack V1 Scope

### 11.1 Goal

Build the most complete first pass that is still practical to iterate on daily.

### 11.2 V1 Includes

- operator review workbench on top of `/admin/tools`
- review timeline UI
- official-link panel
- review-run and verdict persistence
- public trust module on tool detail
- claim flow upgrades on `/submit` or adjacent route
- trust verification module with breakdowns
- external-agent-friendly review packet and logging

### 11.3 V1 Does Not Need

- full autonomous approvals
- perfect scoring science
- public AI leaderboards
- multi-replica distributed review coordination
- advanced training pipeline on day one

### 11.4 Iteration Style

Implementation should favor:

- daily working increments
- visible UI quickly
- mock and real data in parallel where useful
- operator feedback loops over speculative abstraction

---

## 12. Recommended Subagent Work Split

For external coding agents with effectively unlimited parallel workers, split by concern rather than by layer.

### 12.1 Worker A: Operator Console

Own:

- `/admin/tools` interaction redesign
- timeline rendering
- decision panel
- queue readability

### 12.2 Worker B: Public Trust UI

Own:

- tool detail trust sections
- tool card trust markers
- official-link presentation

### 12.3 Worker C: Submit / Claim UX

Own:

- claim-specific fields
- submit/claim branching
- proof explanation copy
- claim status timeline

### 12.4 Worker D: Trust And Data Model

Own:

- migrations
- `tool_official_links`
- `review_runs`
- `review_entries`
- `operator_verdicts`
- `trust_verification.rs`

### 12.5 Worker E: Harness Integration

Own:

- operator harness endpoint expansion
- packet formats
- review-entry persistence
- operator verdict write path

This split reduces collisions and makes external-agent orchestration easier.

---

## 13. Risks And Guards

### 13.1 Risk: Over-trusting AI

Guard:

- human final gate
- append-only verdict history
- explicit dissent capture

### 13.2 Risk: False “Official” Labels

Guard:

- separate link verification table
- strong-proof requirement
- no automatic official promotion

### 13.3 Risk: UI Becoming Too Dense

Guard:

- focus-mode reading model
- center timeline emphasis
- evidence collapsed by default

### 13.4 Risk: Training On Bad Judgments Later

Guard:

- operator verdicts are the gold dataset
- store reason codes and notes
- keep review runs for replay and evaluation

---

## 14. Final Recommendation

Build a **full-stack first version** anchored on the operator review workbench:

- top promotion summary
- left queue rail
- center AI/operator review timeline
- right sticky decision panel

Then connect it to:

- public trust facts on tool detail
- claim-proof collection and verification
- a model-agnostic external agent review harness

This is the strongest path because it supports:

- high-signal discovery
- operator-readable decision making
- public explainable trust
- future evaluation and learning from verified operator judgments

