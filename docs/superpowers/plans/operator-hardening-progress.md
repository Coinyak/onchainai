# Operator Hardening Progress

## Baseline

- Branch: `main` (tracking `origin/main`)
- Dirty files before work: untracked `.claude/`, `.railway-config-pull-5114/`, `docs/superpowers/plans/`, `docs/superpowers/specs/2026-06-26-operator-product-hardening-design.md`
- Disk: target ~30G, free ~25Gi on `/System/Volumes/Data`
- `cargo fmt --check`: PASS
- `cargo test --features ssr`: PASS (147 tests)

## Phase Status

- Phase 0: complete
- Phase 1: complete (2026-06-26)
- Phase 2: complete (2026-06-26)
- Phase 3: complete (2026-06-26)
- Phase 4: complete (2026-06-26)
- Phase 5: complete (2026-06-26)
- Phase 6: complete (2026-06-26)
- Phase 7: complete (2026-06-26)
- Phase 8: complete (2026-06-26)
- Phase 9: complete (2026-06-26)

## Phase 1 Notes

- Docker: pinned `cargo-leptos` 0.3.6, removed SSR-only fallback, added artifact assertions
- Server function: added `ToolListRequest` + `list_tools_v1`; migrated `tools_browser` and `category`
- Scripts: `disk-guard.sh`, `clean-build-artifacts.sh`, `smoke-test.sh`, `browser-smoke.mjs`; deploy smoke wired
- CSS: Strategy A — `style/output.css` is source of truth; added `border-t`, `md:text-[36px]`, `md:text-[16px]`
- Verification: `cargo fmt --check`, `clippy`, `cargo test` (148 tests) PASS; local smoke PASS (release binary)
- Blockers: `./scripts/disk-guard.sh` fails without force when free disk <25GB or target >35GB; `docker build` deferred (low disk); debug `cargo run` stack-overflows on `/` (pre-existing)

## Phase 2 Notes

### Public visibility invariant (Task 2.1)

- Added `PUBLIC_TOOL_WHERE` in `src/server/queries.rs`; `TOOLS_APPROVED_WHERE` aliases it
- Migration `006_operator_hardening.sql`: relevance/safety columns, constraints, RLS policy, `tool_review_events` table
- Updated `Tool` model with review/safety fields; normalizer and deduper use `default_review_fields()`
- All public queries in `functions.rs` and `mcp.rs` use `TOOLS_APPROVED_WHERE` (includes comment/bookmark counts)

### Backfill strategy

**Pragmatic approach:** do NOT mass-mark all rows `accepted`.

1. All rows default to `relevance_status = 'needs_review'`, `install_risk_level = 'medium'`
2. **Approved tools with crypto signals** (regex on name+description: web3, defi, wallet, bridge, mcp, etc.) are backfilled to `relevance_status = 'accepted'` so the catalog stays usable
3. **Approved tools without crypto signals** remain `needs_review` and are hidden from public surfaces until operator re-reviews via admin
4. No automatic override audit for backfill — only new approvals write `tool_review_events`

### Gated review (Task 2.2)

- Added `ReviewToolPayload` and `review_tool` server function with transaction + audit event
- Approval gates: trustworthy URL required; rejected relevance or critical install risk requires `override_reason`; `needs_review` auto-accepts relevance on human approval
- `set_tool_approval` kept as legacy wrapper calling `review_tool`
- Admin UI migrated to `review_tool`; shows relevance/install risk badges; override-approve modal for blocked tools

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (155 tests)

### Blockers

- Migration must be applied in production: `sqlx migrate run` (or Supabase migration) before deploy
- Initial test run failed with "No space left on device"; resolved via `cargo clean` (freed ~89GB)

## Phase 3 Notes

### Crypto relevance scanner (Task 3.1)

- Added `src/crawler/relevance.rs` with `RelevanceAssessment`, adversarial scoring rules, and 8 unit tests
- Wired into `src/crawler/normalizer.rs` — crawled tools get `crypto_relevance_score`, `crypto_relevance_reasons`, `relevance_status`
- Registered module in `src/crawler/mod.rs`
- Scoring: strong chain/DeFi/wallet signals auto-accept (>=70); borderline web3 without evidence → `needs_review`; filesystem/weather/calendar/productivity → `rejected`; keyword stuffing capped without listing evidence

### Install safety scanner (Task 3.2)

- Added `src/install_safety.rs` with `InstallSafetyAssessment`, risk classification, structured Claude config helper, and 12 unit tests
- Registered `install_safety` in `src/lib.rs` (shared client + server)
- Wired into crawler normalizer — populates `install_risk_level`, `install_risk_reasons`, `requires_secret`, `safe_copy_command`
- Updated `src/crawler/mod.rs` `upsert_tools` to persist relevance/safety fields on insert and re-crawl
- Updated `src/components/tool_detail_content.rs` — install warnings for medium/high/critical, no raw `sh -c` JSON config for risky commands, trust panel shows relevance + install risk
- Updated `src/server/mcp.rs` `get_install_guide` — returns `risk_level`, `risk_reasons`, `warning`, `blocked`; critical tools return blocked guidance pending review

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (182 tests)

## Phase 4 Notes

### Operator dashboard (Task 4.1)

- Added `AdminDashboardPage` in `src/pages/admin/dashboard.rs` — stat cards for pending candidates, known updates, high risk installs, open reports (stub 0 when `tool_reports` missing), public tool count, queue counts, crawler source health with last successful crawl per source
- Replaced placeholder `AdminHomePage` in `src/app.rs` with dashboard route at `/admin`
- Added `get_admin_dashboard_stats` server function

### Review queue split (Task 4.2)

- Added six operator queues: `new_candidate`, `known_update`, `needs_manual_research`, `low_relevance`, `reported` (stub empty), `high_risk_install`
- `list_review_queue` server function with SQL WHERE fragments via `review_queue_where`
- Admin tools page uses tab navigation with `?queue=` query param

### Review row data (Task 4.3)

- Rows show name, slug, source, URLs (repo/homepage/npm/MCP), relevance score/status/reasons, install risk/reasons, duplicate candidates (repo/name match stub), stars/last commit, lifecycle/claim state (derived stubs)

### Review actions (Task 4.4)

- Extended `review_tool` for: `needs_info`, `quarantine`, `mark_verified`, `mark_official` — all write `tool_review_events` audit rows
- UI actions: Approve, Reject, Needs info, Quarantine, Mark verified, Mark official (with reason modals)
- Refactored `list_crawler_sources` to share `list_crawler_sources_inner` with dashboard

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (188 tests)

## Phase 5 Notes

### Submission intake (Task 5.1–5.3)

- Migration `007_submission_reports_claims.sql`: `tool_submissions`, `tool_reports`, `tool_claim_requests`, `tools.claim_state` with RLS policies
- Added `src/models/submission.rs` — `ToolSubmission`, `ToolReport`, `ToolClaimRequest`, `ToolSubmissionPayload`
- Added `claim_state` to `Tool` model (unclaimed, claim_pending, claimed, disputed, revoked)
- Server functions: `submit_tool`, `list_my_submissions`, `report_tool`, `request_tool_claim` with auth checks
- Intake rule: minimally plausible submissions accepted; relevance/install scanners run on intake but gate public approval only
- Duplicate detection on slug against pending submissions and existing tools

### `/submit` route (Task 5.3)

- Added `src/pages/submit.rs` — authenticated submission form with validation, scanner-backed intake, user submission status list
- Wired route in `src/app.rs` and `src/pages/mod.rs`
- Updated promo cards, empty state, and top nav to link `/submit`

### Report UI (Task 5.4)

- Added `src/components/tool_listing_actions.rs` — report modal with six reasons wired to `report_tool`
- Integrated on `ToolDetailPage` below tool content

### Claim skeleton (Task 5.5)

- `request_tool_claim` server function stores `tool_claim_requests` and sets `tools.claim_state = claim_pending`
- Claim request modal stub on tool detail page

### Admin queue updates

- `review_queue_where("reported")` now queries tools with open `tool_reports`
- Dashboard `reported` count uses live open-report query (no longer stub 0)

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (197 tests)

## Phase 6 Notes

### Public UX and MCP consistency (Task 6.1–6.4)

- **Homepage promise** (`promo_cards.rs`): CTA is "Suggest a Tool" with operator-review copy; button label "Suggest →"; no x402 paid-registration copy
- **Trust panel** (`tool_detail_content.rs`): shows source, last crawl (`updated_at`), last commit, relevance status, install risk (color badges), verification evidence list, and report listing anchor
- **Empty state** (`empty_state.rs` + `filter_query.rs`): plain-language current filters, clear filters CTA, suggest/submit tool CTA; wired from `tools_browser.rs`
- **MCP output** (`mcp.rs`): all queries use `PUBLIC_TOOL_WHERE` directly; install guide already shares UI risk warnings via `install_safety`
- **CSS** (`style/output.css`): empty-state filter/actions styles, trust evidence/report link, risk/relevance badges, install warning

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS (1 pre-existing `manual_clamp` warning in `operator_harness.rs`)
- `cargo test --features ssr`: PASS (213 tests)

## Phase 7 Notes

### Hermes harness (Task 7.1–7.4)

- Added `src/server/operator_harness.rs` — bounded `OperatorSnapshotV1`, read-only `OperatorRunResponse` with `OperatorRecommendationV1[]`
- Routes (admin-gated): `GET /api/admin/operator/snapshot?queue=pending&limit=25`, `POST /api/admin/operator/run`
- Migration `008_operator_harness.sql`: `operator_tasks`, `agent_action_proposals` tables with admin RLS
- Hard limits enforced: max 25 tools, 3 duplicates/tool, 5 evidence snippets, 500 chars/snippet, 300 chars description, no raw README dump
- Secret redaction: `JWT_SECRET`, `SUPABASE_SERVICE_KEY`, `GITHUB_CLIENT_SECRET`, token prefixes (`ghp_`, `sk-`, etc.)
- Hermes boundary: may propose `approve`, `reject`, `needs_info`, `quarantine`, `outreach`; forbidden `deploy`, `cleanup`, `mark_official`, `mark_verified`, `auth_change`; all recommendations require human approval; run endpoint never mutates `tools`
- Queue alias: `pending` maps to `new_candidate`
- 13 harness unit tests (redaction, limits, recommendations, boundary validation)

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (213 tests)

## Phase 8 Notes

### Rate limit policy (Task 8.1)

- Added `src/server/rate_limit.rs` with per-user governors: submit 5/hour, comment 10/min, bookmark 60/min
- Wired into `submit_tool`, `create_comment`, `toggle_bookmark` server functions
- MCP handler checks separate per-IP limit (100/min) via `check_mcp_ip_rate_limit`
- `src/lib.rs` splits Axum routers: auth 5/min/IP, MCP 100/min/IP, general API 60/min/IP (no double-limiting on auth/MCP)

### Secret redaction (Task 8.2)

- Added `src/server/secret_redaction.rs` — shared `redact_secrets`, `redact_tool_for_admin`, `assert_json_has_no_secrets`
- Harness uses shared redaction module; admin `list_review_queue` and `list_admin_comments` redact before serialization
- Tests cover harness snapshot JSON, admin review queue JSON, and `.env`-style assignments

### Prompt-injection evidence safety (Task 8.3)

- Verified Phase 7 bounds (length/hash/source URL) on `EvidenceSnippetV1`
- Added `evidence_snippets_include_provenance_and_bounds` harness test

### Verification

- `cargo fmt --check`: PASS
- `cargo clippy --features ssr -- -W clippy::all`: PASS
- `cargo test --features ssr`: PASS (225 tests)

## Phase 9 Notes — Final Verification and Deploy Gate

### Fixes applied during gate run

- `src/server/rate_limit.rs`: use `DefaultKeyedRateLimiter<K>` (governor 0.6 generic arity)
- `src/server/secret_redaction.rs`: redact spaced `.env` assignments (`JWT_SECRET = value`)
- `Cargo.toml`: `uuid` feature `js` for wasm32 hydration build
- `src/config.rs`: gate `dotenvy::dotenv()` behind `#[cfg(feature = "ssr")]`
- `src/auth/siwx_client.rs`: import `wasm_bindgen::JsCast` for `dyn_into`
- `src/server/mod.rs`: expose `secret_redaction` to hydrate + SSR (client-safe)
- `src/server/functions.rs`: gate `CommentRow`/`AdminUserRow`/`AdminCommentRow` `into_view` impls with `#[cfg(feature = "ssr")]`
- Freed ~49GB by removing `target/debug` after release build filled disk (kept `target/release` + `target/site`)

### Migrations (operator hardening track)

| Migration | Purpose |
|-----------|---------|
| `006_operator_hardening.sql` | Relevance/safety columns, `PUBLIC_TOOL_WHERE` RLS, `tool_review_events` |
| `007_submission_reports_claims.sql` | `tool_submissions`, `tool_reports`, `tool_claim_requests`, `claim_state` |
| `008_operator_harness.sql` | `operator_tasks`, `agent_action_proposals` (Hermes harness) |

### Verification command outputs (2026-06-26)

```text
$ cargo fmt --check
# exit 0

$ cargo clippy --features ssr -- -W clippy::all
    Finished `dev` profile [unoptimized + debuginfo] target(s)
# exit 0, no warnings

$ cargo test --features ssr
running 225 tests
test result: ok. 225 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ ./scripts/disk-guard.sh
free_disk_gb=29
target_gb=25
# exit 0

$ PATH="$HOME/.cargo/bin:$PATH" cargo leptos build --release
# exit 0 — artifacts:
#   target/release/onchainai          54,269,648 bytes
#   target/site/pkg/onchainai.js          25,406 bytes
#   target/site/pkg/onchainai.wasm   2,207,786 bytes
#   style/output.css                    24,153 bytes

$ SKIP_CRAWLER=1 ./target/release/onchainai  # background
$ ./scripts/smoke-test.sh http://localhost:3000
SMOKE PASS http://localhost:3000

$ node scripts/browser-smoke.mjs http://localhost:3000
BROWSER SMOKE PASS http://localhost:3000
```

### Docker build attempt

```text
$ ONCHAINAI_DISK_GUARD_FORCE=1 ./scripts/disk-guard.sh
ONCHAINAI_DISK_GUARD_FORCE=1 set; continuing

$ docker build -t onchainai .
command not found: docker
```

Docker CLI not available in this environment; disk (29GB free) would otherwise allow an attempt with force flag.

### Remaining risks / deferred items

- **Production deploy not run** — `./scripts/deploy-railway.sh` and production smoke against `https://www.onchain-ai.xyz` deferred (local gates only)
- **Docker build unverified locally** — CLI missing; Railway remote build remains the production gate
- **Migrations must be applied in production** before deploy (`006`, `007`, `008`)
- **Disk hygiene** — full `cargo leptos build --release` needs ~25GB+ `target/`; run `./scripts/clean-build-artifacts.sh` or drop `target/debug` when disk is tight
- **Toolchain PATH** — wasm build requires rustup cargo first (`PATH="$HOME/.cargo/bin:$PATH"`); Homebrew `rustc` alone lacks wasm std
- **Pre-existing** — debug `cargo run` stack-overflow on `/` (use release binary for smoke)
- **Rate limits are in-process** — keyed governors reset on restart; multi-instance deploy needs shared store later

## 2026-06-27 Follow-up — bundle mismatch + pagination

### Context

Production showed `error deserializing server function arguments: missing field filters` — classic SSR binary vs stale WASM/JS bundle skew after partial or mismatched deploys.

### Changes

| Item | Purpose |
|------|---------|
| **Bundle mismatch debug** | Traced deserialization failures to served `target/site/pkg/*` not matching the SSR binary that registered server functions. Confirmed via smoke on `/tools?function=bridge&type=mcp` and Playwright `/pkg/` response checks. |
| **`BUILD_DEPLOY_RULES.md`** | Operator doc: one `cargo leptos build --release` produces both SSR + client artifacts; never deploy SSR-only fallback; run `verify-bundle.sh` before Railway push; smoke release binary locally first. |
| **Pagination limit fix** | `ToolsBrowser` Load more uses **cumulative** `limit = page × 50` with **`offset = 0`** each fetch (`visible_limit_for_page`). Avoids offset-based gaps when sort keys collide. Load more and sort links omit `selected`. |
| **`scripts/verify-bundle.sh`** | Pre-deploy gate: asserts `target/release/onchainai`, `target/site/pkg/onchainai.js`, `.wasm`, and `style/output.css` exist and are from the same build (mtime/size sanity). Wired into `release-build.sh` / deploy runbook. |

### Verification (intended gate)

```text
$ ./scripts/release-build.sh
$ ./scripts/verify-bundle.sh
$ SKIP_CRAWLER=1 ./target/release/onchainai   # background
$ ./scripts/smoke-test.sh http://localhost:3000
$ node scripts/browser-smoke.mjs http://localhost:3000
```

### Remaining

- Apply `BUILD_DEPLOY_RULES.md` on next Railway deploy; run `post-deploy-verify.sh` against production.
- Tablet sidebar default collapsed threshold moving from 768px → **1024px** (`client_storage.rs`) — doc updated in `UI_UX_DESIGN.md` §12.

## 2026-06-27 Browser QA — production click-through + smoke hardening

### Findings (`scripts/click-test.mjs` against `https://www.onchain-ai.xyz`)

| Step | Result | Detail |
|------|--------|--------|
| Home / tools load | PASS | No deserialization errors in body |
| Sidebar brand + filter click | PASS | `?function=bridge` navigation OK |
| Tool cards | PASS | 50 cards on `/tools` |
| Tool logos | PASS | `logo_url` wired: crawler infer (GitHub avatar), `<img>` render + monogram `onerror` fallback |
| Chain strip click | PASS | Filter navigation OK |
| **Load more click** | **FAIL** | Card count **50 → 6** after click (cumulative refetch + cap/validation bug) |
| Tool preview | PASS | `?selected=` opens preview |
| Mobile `/tools` | PASS | No deserialization errors |
| Console | **FAIL** | Multiple **HTTP 500** on same-origin `/api` during load-more |

Root cause: load-more requested `limit > 500` (page × 50 uncapped server-side) and/or stale offset semantics; server rejected with 500 instead of a bounded response. Sort links also kept `selected=` open, closing preview unexpectedly on reorder.

### Fixes applied

| Item | Change |
|------|--------|
| **Load more cap** | `MAX_LIST_TOOLS_LIMIT = 500`; `should_show_load_more()` hides control when next page cannot grow (`480/1000 @ page 10` edge) |
| **Server-fn validation** | `validate_tool_list_request()` rejects out-of-range `limit`/`offset`/filter sizes (no silent clamp → 500) |
| **Cumulative pagination** | `visible_limit_for_page(page)`; `offset = 0` on every fetch |
| **Sort UX** | `build_sort_href` omits `selected` (preview closes on sort change) |
| **Tablet sidebar** | Default collapsed threshold **768px → 1024px** (`sidebar_default_collapsed_for_width`) |
| **RLS alignment** | Migration `015_public_tool_where_containment.sql` — `= ANY(crypto_relevance_reasons)` matches `PUBLIC_TOOL_WHERE` |
| **`browser-smoke.mjs`** | Added load-more markup + click interaction, mobile sidebar collapsed, `.chain-tile-more` visibility (not tool-card `.chain-more`), served CSS non-empty checks |
| **`smoke-test.sh`** | Asserts load-more markup on large `/tools` listing |
| **Featured seed** | `seeds/dev_seed_featured.sql` — 3 active cards after Phase A dev seed |
| **Disk hygiene** | `docs/DISK_MAINTENANCE.md` + `scripts/disk-audit.sh` monthly audit wrapper |

### Verification

```text
$ cargo test --features ssr should_show_load_more public_tool_where
$ ./scripts/smoke-test.sh http://localhost:3000
$ node scripts/browser-smoke.mjs http://localhost:3000
$ node scripts/click-test.mjs http://localhost:3000   # optional prod regression
$ ./scripts/disk-audit.sh
```

### Auth note (`optional_session_result`)

Banned/invalid sessions return `Ok(None)` on optional reads (`get_current_user`, `is_bookmarked`) for hydration-safe UX. **Mutations remain fail-closed** via `require_user` / `require_admin` (`AuthSessionError::Banned` → 401/403). No privilege escalation; explicit “account suspended” messaging belongs on protected actions only.

### Remaining

- Re-run `click-test.mjs` against production after next deploy (load-more 50→N monotonic, no 500s).
- ~~Wire `logo_url` collection + `<img>` render~~ — done (migration 016, `infer_logo_url`, `logo_url_is_safe_for_img`, upsert sanitize, `onerror` monogram fallback, `referrerpolicy=no-referrer`).
- Seed production featured cards via `/admin/featured` or operator SQL.