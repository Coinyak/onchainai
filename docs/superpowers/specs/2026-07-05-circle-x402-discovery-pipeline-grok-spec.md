# Circle / x402 Discovery Pipeline — Grok Spec (10-Reviewer Consensus)

**Status:** Implementation-ready spec for Grok `/goal` sessions  
**Source plan:** `/Users/hoyeon/.claude/plans/https-t-co-unixjotajy-squishy-star.md`  
**Review:** 10 parallel subagents (codebase, security, crawler, schema, ops, x402 policy, PR DAG, tests, docs, Grok harness)  
**Date:** 2026-07-05

---

## 0. Executive summary

**Problem:** Circle for Agents (`agents.circle.com`) and most x402 Bazaar sellers are missing from the catalog because crawlers are tag/topic pull-only and `circlefin` is absent from `FIRST_PARTY_ORGS`.

**Solution:** Phase A (Circle seed + verify) + Phase B (shared vendor list, vendor-org crawler, Bazaar crawler, x402 plumbing).

**Grok execution model:** **One `/goal` per PR** (6 goals). Do **not** run the full program as a single goal — skeptics will refute.

**Skills to use:**

| When | Skill / command |
|------|-----------------|
| Each PR implementation | `/goal` with objective from §6 |
| Parallel PR-2 + PR-3 | Two `/goal` sessions or `/execute-plan` after this spec |
| Doc-only PR-6 | `/goal` or direct agent |
| Pre-merge review | `/review` or `code-review` skill |
| Post-implementation check | `/check-work` |
| UI untouched | Skip `onchainai-ui-workflow` |

---

## 1. Ten-reviewer consensus

### 1.1 Confirmed (keep as-is)

| Claim | Evidence |
|-------|----------|
| `RawTool` lacks `pricing` / `x402_price` / `x402_endpoint` | `src/crawler/normalizer.rs` |
| `normalize()` hardcodes `pricing: "free"` | same |
| `upsert_tools` overwrites `pricing` unconditionally on conflict | `src/crawler/mod.rs:142` |
| `x402_endpoint` column exists but crawler upsert omits it | migration 028; `mod.rs` INSERT list |
| `circlefin` missing from `FIRST_PARTY_ORGS` (45 entries today, not ~50) | `scripts/verify-tool-official.mjs:49-95` |
| `seed-cex-tools.mjs` pattern (`tool()` + `runSeed()`) is correct template | `scripts/seed-cex-tools.mjs` |
| SQL migrations **0** for this program (schema ready) | 001/017/028 already have x402 columns |
| Bazaar L2 deferred in roadmap; OPEN_LISTING L2 is normative when implemented | `docs/X402_OPEN_LISTING_SPEC.md` §L2 |
| `crypto_registry_source()` must **not** include bazaar | `src/crawler/relevance.rs:239` |

### 1.2 Contradictions in source plan (corrected)

| # | Source plan says | Reality / fix |
|---|------------------|---------------|
| C1 | `prepare_crawled_tools(&raws, true)` = per-source forced pending | **Wrong.** Second arg is global `require_tool_approval` from `site_settings`. Per-source pending needs new `persist_crawl_results_gated()` (PR-3). |
| C2 | PR-2 regression: `verify-tool-official.mjs circle-gateway` | **`circle-gateway` does not exist** until PR-1 seeds it. Pre-PR-1 use `bob-gateway-cli` for regression, or run regression **after** PR-1 merge. |
| C3 | A1 inline `FIRST_PARTY_ORGS` then B0 JSON migration | **Merge conflict.** Fold `circlefin` into B0 only; skip standalone A1 if B0 lands same stack. |
| C4 | `RawTool ..Default::default()` mechanical fix | **`Default` not implemented** on `RawTool`. Add `impl Default` + ~18 call sites. |
| C5 | X402_ROADMAP "X9" for pending gate (ambiguous) | **Correct anchor:** `X402_ROADMAP.md` §10.3.C **X9** (Bazaar L2 crawl → operator queue). **Wrong:** activation-spec X9 (referral stats UI). |
| C6 | `circle-official-use-arc` duplicate with `circlefin/skills` | **Not in repo.** Replace with live `search_tools` / DB pre-check before seeding. |
| C7 | OPERATOR_GUIDE "4 crawler sources" | **7** discovery sources today (`CRAWLER_SOURCE_DEFS`: cryptoskill, github, mcp-registry, npm, vendor_orgs, bazaar, web3-mcp-hub) + `sync_stars` maintenance; fixed in PR-6 docs. |
| C8 | Seed sets `official` | **Wrong.** `seed-tool-lib.mjs` sets `status='community'`; promotion only via `verify-tool-official.mjs --apply`. |

### 1.3 Additions required (not in source plan)

| ID | Addition | PR |
|----|----------|-----|
| A1 | Fix `mcp-registry` in `CRAWLER_SOURCE_DEFS`, `validate_trigger_crawler_source`, `default_source_registry_url` | PR-4/5 (B2c) |
| A2 | `persist_crawl_results_gated()` — always `pending` for vendor_orgs + bazaar regardless of `site_settings` | PR-3 |
| A3 | Upsert clobber guard: preserve `pricing='x402'`, `relevance_status` when `last_reviewed_at` set, trusted-row `homepage`/`repo_url` | PR-3 |
| A4 | Named integration tests: `upsert_x402_clobber_guard_*`, `persist_crawl_results_gated_*`, `bazaar_grouping_*`, `vendor_orgs_slug_rename_*` | PR-3, PR-4, PR-5 |
| A5 | Bazaar: `probe_x402_endpoint` at ingest (reuse 028 stack); `referral_enabled=false`; live probe → `relevance_status='accepted'` | PR-5 |
| A6 | Bazaar dedupe by normalized `x402_endpoint`, not `repo_url` only | PR-5 |
| A7 | `upsert_tools` must write `x402_endpoint` on INSERT/UPDATE for crawler path | PR-3 |
| A8 | Slug collision: refuse metadata overwrite on `official`/`verified`/claimed rows | PR-3 or PR-4 |
| A9 | Reconcile `SEED_ENV=prod-curate` with `SEED_DATA.md` (operator curation lane, not dev seed) | PR-6 docs |
| A10 | OPERATOR_GUIDE §5 link-drop workflow + crawler count fix | PR-6 |
| A11 | Admin crawler UI sends UUID not source name — **existing bug**; document curl workaround until fixed | PR-6 or hotfix |
| A12 | Optional `scripts/coverage-report.mjs` for `zero_coverage` org alarm | PR-2 |

### 1.4 Non-goals (unchanged + expanded)

- B1c GitHub Code Search sweep (separate future goal)
- SQL migrations
- UI / Leptos changes (`ui-change-gate` not required unless admin crawler fix touches `frontend/`)
- Full 23k Bazaar ingest (cap 300 items / 100 hosts per run)
- X API / Sourcegraph / Quod AI crawlers
- Manual `tools.status` writes
- Auto-scheduled `verify-tool-official --scan --apply`
- Custody / payment proxy for third-party x402

---

## 2. Corrected PR stack

```
PR-1 (Circle seed)     ──┐
                         ├──► PR-2 (vendor-orgs.json) ──► PR-4 (vendor_orgs.rs)
PR-3 (x402 plumbing)  ──┴──► PR-5 (bazaar.rs)
                                    │
                                    └──► PR-6 (docs)
```

| Order | PR | Blocks | Can parallel with |
|-------|-----|--------|-------------------|
| 1 | **PR-1** | — | — |
| 2 | **PR-2** | PR-1 (for circle-gateway regression) | PR-3 |
| 3 | **PR-3** | — | PR-2 |
| 4 | **PR-4** | PR-2, PR-3 | — |
| 5 | **PR-5** | PR-3 | — |
| 6 | **PR-6** | PR-4, PR-5 | — |

**Push:** `[skip ci]` on all PRs (AGENTS.md).

---

## 3. Global hard rules (all goals)

1. Never `UPDATE tools SET status=…` by hand — only `verify-tool-official.mjs --apply`.
2. Never `--scan --apply` without `--i-understand-bulk`.
3. Never set `payment_verified` / `x402_endpoint_verified` / `price_verified` from crawler or seed.
4. Never auto-approve vendor_orgs or bazaar rows (use gated pending).
5. Never add bazaar to `crypto_registry_source()`.
6. Never ingest full Bazaar catalog.
7. Run `git status --short` before edits.
8. Local gates: `cargo test --features ssr`, `clippy`, `fmt --check`, `./scripts/agent-harness-check.sh`.
9. Prod seed: dry-run first; apply only with `SEED_ENV=prod-curate` + operator ack.
10. Capture all command output under `{SCRATCH}/` for skeptic verification.

---

## 4. Shared technical contracts

### 4.1 `scripts/vendor-orgs.json` (PR-2)

```json
{
  "version": 1,
  "orgs": [
    { "github": "circlefin", "team": "Circle", "crawl": true, "npm_scopes": ["@circle-fin"] }
  ]
}
```

- Migrate all 45 `FIRST_PARTY_ORGS` entries + `crcl-main`.
- `crawl: true` only for ~25 crypto vendors; `github`, `openai`, `anthropics`, `discord` → `crawl: false`.
- `verify-tool-official.mjs`: `readFileSync` loader.
- Rust: `include_str!` + `serde` + `OnceLock` (no new deps).
- Test: JSON parses; `circlefin` present.

### 4.2 `persist_crawl_results_gated()` (PR-3)

```rust
// Always approval_status = "pending" for vendor_orgs + bazaar sources.
// Ignores site_settings.require_tool_approval.
pub async fn persist_crawl_results_gated(pool, name, url, raws) { ... }
```

Unit test: `persist_crawl_results_gated_respects_force_pending`.

### 4.3 Upsert clobber guard (PR-3)

```sql
pricing = CASE
  WHEN tools.pricing IN ('x402','paid','freemium') THEN tools.pricing
  ELSE EXCLUDED.pricing END,
x402_price = COALESCE(tools.x402_price, EXCLUDED.x402_price),
x402_endpoint = COALESCE(tools.x402_endpoint, EXCLUDED.x402_endpoint),
relevance_status = CASE
  WHEN tools.last_reviewed_at IS NOT NULL THEN tools.relevance_status
  ELSE EXCLUDED.relevance_status END
```

Test: `upsert_x402_clobber_guard_preserves_self_listing_metadata`.

### 4.4 Crawler wiring checklist (PR-4, PR-5)

Each new source must touch:

1. `src/crawler/sources/mod.rs` — `pub mod` + `default_crawlers()`
2. `src/crawler/scheduler.rs` — `CRAWLER_JOB_SPECS` + `SCHEDULER_JOB_COUNT`
3. `src/crawler/mod.rs` — `default_source_registry_url()` + `trigger_source()`
4. `src/server/functions/crawler_admin.rs` — `CRAWLER_SOURCE_DEFS`, `validate_trigger_crawler_source`, `default_schedule_minutes_for_source`
5. `src/server/functions/function_tests.rs` — source name tests
6. **Repair `mcp-registry`** in items 4–5 (pre-existing gap)

### 4.5 Bazaar API (PR-5)

- URL: `GET https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources`
- `MAX_PAGES=3`, `limit=100` → 300 items max
- Spam floor: `l30DaysUniquePayers >= 5`
- Host grouping; representative item = max payers, prefer resource without `:param`
- `tool_type: "x402"`, `pricing: "x402"`, `referral_enabled: false`
- Chain map: 8453→base, 1→ethereum, 137→polygon, 42161→arbitrum, 10→optimism, 43114→avalanche; drop testnet-only
- Cron: `0 20 */6 * * *` (6h)
- Run cap: 100 hosts per run
- Probe at ingest via existing `probe_x402_endpoint` (028 stack)

### 4.6 vendor_orgs crawler (PR-4)

- `GET /orgs/{org}/repos?per_page=100&type=public&sort=pushed`
- Filters: no fork/archived; pushed ≤18 months; `MIN_STARS=3`; top 25/org
- Slug guard: names <5 chars or in `{skills,docs,examples,sdk,api,contracts,cli,core}` → `"{org}-{repo}"`
- Pre-query: exclude existing `repo_url` values
- Cron: `0 45 3 * * *` (daily ~03:45 UTC)
- `github_client` / `parse_datetime` → `pub(super)` in `github.rs`

---

## 5. Circle seed manifest (PR-1)

Resolve npm/homepage with `npm view` + `curl -sfI` **before** writing rows.

| slug | type | function | repo | npm | notes |
|------|------|----------|------|-----|-------|
| circle-agent-stack | sdk | payments | circlefin/agent-stack-starter-kits | — | homepage agents.circle.com |
| circle-x402-batching | sdk | payments | — | @circle-fin/x402-batching | **community** (no repo) |
| circle-gateway | api | payments | circlefin/evm-gateway-contracts | — | |
| circle-cctp-v2 | api | bridge | circlefin/evm-cctp-contracts | — | multi-chain |
| circle-cctp-provider-sdk | sdk | bridge | TBD from npm | @circle-fin/provider-cctp-v2 | |
| circle-dev-controlled-wallets | sdk | wallet | TBD | @circle-fin/developer-controlled-wallets | requires_secret |
| circle-user-controlled-wallets | sdk | wallet | TBD | @circle-fin/user-controlled-wallets | |
| circle-modular-wallets | sdk | wallet | circlefin/modularwallets-web-sdk | — | |
| circle-paymaster | api | payments | — | — | homepage TBD (curl 200) |
| circle-api-node-sdk | sdk | payments | circlefin/circle-nodejs-sdk | @circle-fin/circle-sdk | |
| usdc-stablecoin-contracts | sdk | dev-tool | circlefin/stablecoin-evm | — | asset_class: stablecoins |

**Exclude:** `circlefin/skills` (check live DB for `skills` slug collision first).

---

## 6. Grok `/goal` objectives (copy-paste)

### Goal 1 — PR-1

```
/goal PR-1: Add circlefin to FIRST_PARTY_ORGS, create seed-circle-agent-tools.mjs (11 slugs), dry-run + prod-curate apply + verify --apply for repo slugs; search_tools circle≥10 CCTP≥2.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §5–6 Goal 1.
```

**Acceptance criteria:**
1. `circlefin: "Circle"` in `FIRST_PARTY_ORGS`.
2. `scripts/seed-circle-agent-tools.mjs` exists; 11 slugs per §5; all URLs curl-200 at authoring.
3. `{SCRATCH}/seed-dry-run.log` + `{SCRATCH}/seed-apply.log` (SEED_ENV=prod-curate).
4. Repo slugs: `{SCRATCH}/verify-<slug>.log` shows `official` via first-party path.
5. `circle-x402-batching` stays `community` (documented in `{SCRATCH}/verify-summary.log`).
6. `search_tools` "circle" ≥10, "CCTP" ≥2 — JSON in `{SCRATCH}/`.
7. `cargo test/clippy/fmt --features ssr` + `agent-harness-check.sh` PASS.

### Goal 2 — PR-2

```
/goal PR-2: Create scripts/vendor-orgs.json (45+ orgs, circlefin, crawl flags), wire verify-tool-official.mjs + Rust include_str test; optional coverage-report.mjs; regression verify dry-run on circle-gateway post-PR-1.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §4.1 Goal 2.
```

**Acceptance criteria:**
1. `vendor-orgs.json` parses; 45+ orgs; `circlefin` with `crawl: true`.
2. `verify-tool-official.mjs` loads JSON; inline map removed.
3. Rust test: JSON includes `circlefin`.
4. `node scripts/verify-tool-official.mjs circle-gateway` dry-run → `official` (requires PR-1 merged).
5. `cargo test --features ssr` PASS.

### Goal 3 — PR-3

```
/goal PR-3: RawTool x402 fields + Default impl; normalize reflects raw; upsert clobber guard + x402_endpoint column; persist_crawl_results_gated + unit/integration tests.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §4.2–4.3 Goal 3.
```

**Acceptance criteria:**
1. `RawTool` has `pricing`, `x402_price`, `x402_endpoint: Option<String>` + `Default`.
2. `upsert_tools` writes `x402_endpoint`; clobber guard per §4.3.
3. `persist_crawl_results_gated()` forces pending regardless of site_settings.
4. Tests listed in §7.1 pass and appear in `cargo test --features ssr -- --list`.
5. `cargo test/clippy/fmt --features ssr` PASS.

### Goal 4 — PR-4

```
/goal PR-4: Implement vendor_orgs.rs crawler, wire all 6 admin/scheduler touchpoints + fix mcp-registry gap, wiremock tests, repo_url exclusion query, slug rename guard.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §4.4 §4.6 Goal 4.
```

**Acceptance criteria:**
1. `vendor_orgs.rs` + wiremock tests (fork/archived/low-star/rename).
2. All §4.4 wiring complete including **mcp-registry repair**.
3. Uses `persist_crawl_results_gated`.
4. `vendor_orgs_slug_rename_policy` test passes.
5. `cargo test/clippy/fmt --features ssr` PASS.

### Goal 5 — PR-5

```
/goal PR-5: Implement bazaar.rs (CDP discovery, spam floor, host grouping, x402 mapping, probe at ingest), wire + wiremock tests, 6h cron.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §4.5 Goal 5.
```

**Acceptance criteria:**
1. `bazaar.rs` with wiremock: grouping, payers floor, chain map, testnet drop.
2. `referral_enabled=false`, `pricing/type=x402`, no registry bonus.
3. `bazaar_grouping_collapses_same_merchant_resources` test passes.
4. Uses `persist_crawl_results_gated`.
5. `cargo test/clippy/fmt --features ssr` PASS.
6. Post-deploy (operator): manual trigger once; pending queue visible in `/admin/tools`.

### Goal 6 — PR-6

```
/goal PR-6: Update OPERATOR_GUIDE (7 discovery sources + star sync, link-drop §5, pending review, verify scan runnerbook), MVP_DESIGN §3, X402_ROADMAP §3.2 Bazaar done note, SEED_DATA prod-curate note.

Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md §6 Goal 6.
```

**Acceptance criteria:**
1. OPERATOR_GUIDE: 7 discovery sources (`CRAWLER_SOURCE_DEFS`) + `sync_stars` maintenance; link-drop workflow §5.
2. MVP_DESIGN §3: vendor_orgs + bazaar rows.
3. X402_ROADMAP §3.2: Bazaar L2 marked done (with Phase 2 footnote if deferred).
4. SEED_DATA: `prod-curate` operator lane documented.
5. No remaining "4 crawler sources" without historical note.

---

## 7. Test matrix (skeptic-proof)

### 7.1 Required named tests

| Test name | PR |
|-----------|-----|
| `upsert_x402_clobber_guard_preserves_self_listing_metadata` | PR-3 |
| `persist_crawl_results_gated_respects_force_pending` | PR-3 |
| `vendor_orgs_slug_rename_policy` | PR-4 |
| `bazaar_grouping_collapses_same_merchant_resources` | PR-5 |

### 7.2 Evidence commands (every goal)

```bash
cargo test --features ssr 2>&1 | tee {SCRATCH}/full-tests.log
cargo test --features ssr -- --list | grep -E 'clobber|gated|bazaar|vendor_orgs'
cargo clippy --features ssr -- -W clippy::all 2>&1 | tee {SCRATCH}/clippy.log
cargo fmt --check 2>&1 | tee {SCRATCH}/fmt.log
./scripts/agent-harness-check.sh 2>&1 | tee {SCRATCH}/agent-harness.log
```

### 7.3 Post-deploy (Goals 4–5, operator)

```bash
./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
# Manual trigger (until admin UI fixed):
curl -X POST "$API_URL/api/v2/admin/crawler/trigger" \
  -H "Cookie: $ADMIN_COOKIE" -H "Content-Type: application/json" \
  -d '{"source":"vendor-orgs"}'   # or "bazaar"
```

---

## 8. Risk register (reviewer synthesis)

| Risk | Mitigation | Owner PR |
|------|------------|----------|
| Org sweep flood | stars/recency/cap + forced pending | PR-4 |
| Bazaar wash traffic | uniquePayers≥5 + host cap | PR-5 |
| Slug hijacking | rename guard + trusted-row metadata guard | PR-3/4 |
| x402→free reset | clobber guard | PR-3 |
| Seed↔crawler duplicate | repo_url exclusion query | PR-4 |
| GitHub rate limit | GITHUB_API_TOKEN; ~40 req/day | ops |
| Prod seed accident | dry-run + prod-curate gate | PR-1 |
| Admin trigger broken | curl workaround; fix in PR-4/6 | PR-6 |
| Skeptic refute on vague goals | §6 acceptance criteria + `{SCRATCH}/` logs | all |

---

## 9. Document lineage

| Document | Action |
|----------|--------|
| `docs/OPERATOR_GUIDE.md` | PR-6: §5 link-drop, crawler count, verify runnerbook |
| `docs/MVP_DESIGN.md` §3 | PR-6: +2 sources |
| `docs/X402_ROADMAP.md` §3.2 | PR-6: Bazaar complete checkbox |
| `docs/SEED_DATA.md` | PR-6: prod-curate lane |
| `docs/X402_OPEN_LISTING_SPEC.md` §L2 | Reference only; optional normative table add |
| `AGENTS.md` | No change (rules already cover status + x402) |

---

## 10. Quick start for operator

1. Read this spec + `AGENTS.md`.
2. `/goal` Goal 1 (PR-1) in a **new Grok session**.
3. `/goal status` to monitor; `/btw` for mid-course corrections.
4. After Goal 1 verified, proceed Goals 2–3 (can parallel).
5. Goals 4–5 after Goal 3 merges.
6. Goal 6 last.
7. Railway deploy between 3→4 and 4→5; manual crawler trigger + pending queue review.

**First command:**

```
/goal PR-1: Add circlefin to FIRST_PARTY_ORGS, create seed-circle-agent-tools.mjs (11 slugs), dry-run + prod-curate apply + verify --apply for repo slugs; search_tools circle≥10 CCTP≥2. Spec: docs/superpowers/specs/2026-07-05-circle-x402-discovery-pipeline-grok-spec.md
```