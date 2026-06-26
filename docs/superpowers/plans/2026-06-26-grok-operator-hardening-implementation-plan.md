# Grok Operator Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make OnchainAI stable to deploy, safe to publish, easy to operate, and ready for Hermes-assisted management without allowing agents to approve unsafe listings.

**Architecture:** Implement this as an incremental hardening track, not a rewrite. The first work locks down build/deploy/UI stability, then adds a public visibility invariant, then builds review, submission, safety, and Hermes harness layers on top. Grok should spawn 10 read/write subagents with disjoint ownership and an integration lead that reviews each phase before the next phase starts.

**Tech Stack:** Rust, Leptos SSR/hydration, Axum, sqlx, Supabase Postgres/RLS, tokio-cron-scheduler, Railway Docker deploy, shell scripts, optional Playwright smoke checks.

---

## Objective Review Summary

Six independent reviewers checked the current repo and the hardening spec. Their objective findings are treated as plan requirements.

### Blocking Findings

- Docker currently hides `cargo leptos build --release` failure with `|| echo`, so Railway can deploy a server without a valid matching WASM/JS bundle.
- Hydration is enabled only by checking whether `target/site/pkg/onchainai.js` exists, which can accept stale bundles.
- `ListTools` uses a positional server-function signature that can break cached clients with `missing field filters`.
- UI changes can silently fail because `style/output.css` is a manual Tailwind-like stub, not a generated Tailwind pipeline.
- `target/site/pkg/onchainai.css` is currently 0 bytes, while the served CSS path is manually overridden to `style/output.css`.
- `approval_status = 'approved'` is the only public filter today; it does not include crypto relevance, install safety, quarantine, or review policy.
- Supabase RLS currently permits public `SELECT` on all `tools`, including pending/rejected rows if accessed directly.
- `set_tool_approval` can approve a tool without relevance/safety/trust gates or audit event.
- Raw install commands are embedded into UI/MCP install guides, including `sh -c` style config.
- Hermes must never directly approve/reject listings; it should only propose recommendations using bounded snapshots.

### Product Corrections

- Intake should be permissive; publication should be strict. A plausible submission can enter the queue even if crypto relevance is uncertain.
- Add acquisition metrics: discovered, reviewed, approved, viewed, clicked/installed, MCP queried, claimed, updated, stale.
- Add listing lifecycle states: candidate, pending, public_unclaimed, claimed, flagged, stale, deprecated, delisted.
- Claim flow needs governance: pending, claimed, disputed, revoked; official badge remains separate from claim.
- Outreach copy must say "unofficial listing" until a project claims or verifies it.

---

## Grok Handoff Prompt

Use this prompt when handing work to Grok:

```text
You are Grok working in /Users/love/OnchainAI.

Read first:
- AGENTS.md
- docs/superpowers/specs/2026-06-26-operator-product-hardening-design.md
- docs/superpowers/plans/2026-06-26-grok-operator-hardening-implementation-plan.md
- docs/MVP_DESIGN.md
- docs/UI_UX_DESIGN.md
- docs/SECURITY.md
- DESIGN.md

Goal:
Implement the operator product hardening track incrementally. Do not rewrite the app.

Spawn exactly 10 subagents with the roles in this plan. Each subagent must own a disjoint file/module responsibility. The Integration Lead coordinates phase gates and prevents overlapping edits.

Rules:
- Work on main unless the user explicitly changes this.
- Do not commit .env or expose secrets.
- Do not revert user changes.
- All cargo commands touching server code use --features ssr.
- Add/update tests for every changed behavior.
- Keep public UI text English.
- Admin routes and harness routes must be server-side admin-gated.
- Risky actions require audit trails and human approval.
- Production Docker builds must fail if SSR, JS, WASM, or CSS artifacts are invalid.
- Public listing visibility must be enforced in DB/RLS, server API, and MCP.
- Make small commits by phase if committing is requested.

Execution order:
Phase 0 through Phase 9 in this plan. Integration Lead must review outputs after each phase before the next phase starts.

Final deliverable:
- Summary by phase
- Files changed
- Migrations added
- Tests run with exact commands and results
- Local and production smoke test results
- Remaining risks or explicitly deferred non-goals
```

---

## 10 Grok Subagents

### 1. Baseline & Integration Lead

**Ownership:** branch state, phase sequencing, integration review, final verification.

**Must not edit:** feature code unless resolving integration conflicts.

**Responsibilities:**
- Record baseline `git status --short --branch`.
- Record current test results before changes.
- Ensure no two subagents edit the same file without handoff.
- Run phase gates.
- Maintain a short `docs/superpowers/plans/operator-hardening-progress.md` if the work spans multiple sessions.

### 2. Smoke/Build Compatibility Agent

**Ownership:**
- `Dockerfile`
- `scripts/smoke-test.sh`
- `scripts/deploy-railway.sh`
- `src/app.rs`
- `src/lib.rs`
- build-id artifact if added

**Responsibilities:**
- Remove Docker SSR-only fallback for production.
- Pin `cargo-leptos` version.
- Assert build artifacts exist and are non-empty.
- Add curl + browser smoke checks.
- Add cache policy for non-hashed `/pkg` assets or introduce a build-id manifest.

### 3. Disk Hygiene Agent

**Ownership:**
- `scripts/disk-guard.sh`
- `scripts/clean-build-artifacts.sh`
- `AGENTS.md` disk hygiene section if needed
- deploy/build docs updates

**Responsibilities:**
- Add guard before heavy builds.
- Add safe cleanup with dry-run and symlink checks.
- Avoid deleting `.env`, migrations, source, docs, or user data.

### 4. Schema & Models Agent

**Ownership:**
- `migrations/006_operator_hardening.sql`
- `src/models/tool.rs`
- new model structs
- `src/server/queries.rs`
- RLS policy updates

**Responsibilities:**
- Add relevance, install safety, quarantine, review, submission, report, operator task, and proposal schema.
- Enforce `PUBLIC_TOOL_WHERE`.
- Backfill existing approved rows safely.

### 5. Relevance Scanner Agent

**Ownership:**
- `src/crawler/relevance.rs`
- `src/crawler/mod.rs`
- `src/crawler/normalizer.rs`
- crawler tests

**Responsibilities:**
- Score crypto/onchain relevance.
- Prevent generic MCP keyword stuffing from passing.
- Feed relevance status into pending/public gates.

### 6. Install Safety Agent

**Ownership:**
- `src/install_safety.rs`
- install-related model fields
- `src/components/tool_detail_content.rs`
- `src/server/mcp.rs`
- scanner tests

**Responsibilities:**
- Parse/rank install commands.
- Stop generating `sh -c` configs from raw install strings.
- Disable or warn on critical/high risk commands.

### 7. Admin Console Agent

**Ownership:**
- `src/pages/admin/*`
- `src/server/functions.rs` admin review functions
- admin components

**Responsibilities:**
- Convert admin home into an operations dashboard.
- Replace `set_tool_approval` with gated review action.
- Add review history and override reasons.

### 8. Submission/Growth Agent

**Ownership:**
- `src/pages/submit.rs`
- submission server functions
- tool submission/report/claim skeleton UI
- route wiring in `src/app.rs`

**Responsibilities:**
- Add `/submit`.
- Add duplicate detection.
- Add update suggestions and reports.
- Add claim skeleton and outreach-friendly listing state.

### 9. Public UX/MCP Agent

**Ownership:**
- `src/components/promo_cards.rs`
- `src/components/empty_state.rs`
- `src/components/tool_card.rs`
- `src/components/tool_detail_content.rs`
- public MCP output shape if not owned by Install Safety
- `style/output.css` or replacement CSS pipeline

**Responsibilities:**
- Fix public copy promises.
- Add trust/risk indicators.
- Fix empty state.
- Fix UI changes not reflecting by establishing a real CSS strategy.

### 10. Hermes Harness & QA Agent

**Ownership:**
- `src/server/operator_harness.rs`
- harness route wiring
- operator task/proposal models
- harness tests
- final QA matrix

**Responsibilities:**
- Add bounded snapshots.
- Add read-only recommendation runs.
- Add secret redaction.
- Ensure Hermes proposals never mutate public listing state directly.

---

## Phase 0: Baseline Freeze

**Files:**
- Read: `AGENTS.md`
- Read: `Cargo.toml`
- Read: `Dockerfile`
- Read: `docs/superpowers/specs/2026-06-26-operator-product-hardening-design.md`
- Create if long-running: `docs/superpowers/plans/operator-hardening-progress.md`

- [ ] **Step 0.1: Record branch and dirty state**

Run:

```bash
git status --short --branch
```

Expected:

- Current branch is `main`.
- Existing unrelated untracked directories may include `.claude/` and `.railway-config-pull-5114/`.
- Do not delete or stage unrelated files.

- [ ] **Step 0.2: Record baseline target size**

Run:

```bash
du -sh target 2>/dev/null || true
df -h .
```

Expected:

- Output is copied into the progress note or final summary.
- If free disk is under 25GB, stop before heavy builds.

- [ ] **Step 0.3: Run baseline formatting check**

Run:

```bash
cargo fmt --check
```

Expected:

- PASS, or record pre-existing formatting failure before any edits.

- [ ] **Step 0.4: Run baseline tests**

Run:

```bash
cargo test --features ssr
```

Expected:

- PASS, or record pre-existing failures with exact test names.

- [ ] **Step 0.5: Create phase progress note if needed**

If this work is not completed in one run, create:

```markdown
# Operator Hardening Progress

## Baseline

- Branch:
- Dirty files before work:
- Disk:
- `cargo fmt --check`:
- `cargo test --features ssr`:

## Phase Status

- Phase 0:
- Phase 1:
- Phase 2:
- Phase 3:
- Phase 4:
- Phase 5:
- Phase 6:
- Phase 7:
- Phase 8:
- Phase 9:
```

Commit only if the user asks for commits.

---

## Phase 1: Build, Deploy, CSS, and Smoke Stability

This phase comes before product features. It addresses the user's recurring issue:
UI changes do not reliably appear and deploys can look successful while the app is broken.

### Task 1.1: Make Docker builds fail on invalid Leptos artifacts

**Files:**
- Modify: `Dockerfile`

- [ ] **Step 1: Remove build failure masking**

Replace:

```dockerfile
RUN cargo leptos build --release 2>&1 | tee /tmp/leptos-build.log \
    || echo "WASM bundle build skipped; SSR-only mode"
```

With:

```dockerfile
RUN cargo leptos build --release 2>&1 | tee /tmp/leptos-build.log
```

- [ ] **Step 2: Pin cargo-leptos**

Replace:

```dockerfile
RUN cargo install cargo-leptos --locked
```

With a pinned version verified by the implementer:

```dockerfile
ARG CARGO_LEPTOS_VERSION=0.2.43
RUN cargo install cargo-leptos --version "${CARGO_LEPTOS_VERSION}" --locked
```

If `0.2.43` does not support Leptos 0.8 in this repo, choose the latest compatible version and record it in the final summary.

- [ ] **Step 3: Assert required artifacts**

After `cargo leptos build --release`, add:

```dockerfile
RUN test -s /app/target/release/onchainai \
    && test -s /app/target/site/pkg/onchainai.js \
    && test -s /app/target/site/pkg/onchainai.wasm
```

If production still intentionally serves `style/output.css`, assert that too:

```dockerfile
RUN test -s /app/style/output.css
```

- [ ] **Step 4: Build Docker locally when disk guard exists**

Run after Task 1.3:

```bash
./scripts/disk-guard.sh
docker build -t onchainai .
```

Expected:

- Docker build fails if WASM/JS build fails.
- No SSR-only image is produced silently.

### Task 1.2: Introduce stable tool-list server function

**Files:**
- Modify: `src/server/functions.rs`
- Modify: `src/components/tools_browser.rs`
- Modify: `src/pages/category.rs`

- [ ] **Step 1: Add request struct**

Add near `ToolFilters`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolListRequest {
    pub sort: String,
    pub offset: i64,
    pub limit: i64,
    pub filters: ToolFilters,
    pub query: Option<String>,
}
```

- [ ] **Step 2: Add `list_tools_v1` server function**

Add:

```rust
#[server(ListToolsV1, "/api")]
pub async fn list_tools_v1(req: ToolListRequest) -> Result<Vec<Tool>, ServerFnError> {
    list_tools(req.sort, req.offset, req.limit, req.filters, req.query).await
}
```

Keep the old `list_tools` function during compatibility migration.

- [ ] **Step 3: Migrate browser call**

In `src/components/tools_browser.rs`, replace:

```rust
list_tools(sort, 0, 50, filters, search_q)
```

With:

```rust
list_tools_v1(ToolListRequest {
    sort,
    offset: 0,
    limit: 50,
    filters,
    query: search_q,
})
```

Update imports.

- [ ] **Step 4: Migrate category call**

In `src/pages/category.rs`, call `list_tools_v1(ToolListRequest { ... })` instead of positional `list_tools`.

- [ ] **Step 5: Add tests**

Add or update tests in `src/server/functions.rs`:

```rust
#[test]
fn tool_list_request_serializes_filters_field() {
    let req = ToolListRequest {
        sort: "hot".into(),
        offset: 0,
        limit: 50,
        filters: ToolFilters {
            function: vec!["bridge".into()],
            ..Default::default()
        },
        query: Some("mcp".into()),
    };
    let json = serde_json::to_value(&req).expect("serialize request");
    assert!(json.get("filters").is_some());
    assert_eq!(json["sort"], "hot");
}
```

- [ ] **Step 6: Verify**

Run:

```bash
cargo test --features ssr tool_list_request_serializes_filters_field
cargo test --features ssr
```

Expected:

- Tests pass.
- Existing old `ListTools` remains available during rollout.

### Task 1.3: Add disk hygiene scripts

**Files:**
- Create: `scripts/disk-guard.sh`
- Create: `scripts/clean-build-artifacts.sh`
- Modify: `AGENTS.md` if command docs need correction

- [ ] **Step 1: Create `scripts/disk-guard.sh`**

Use this behavior:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MIN_FREE_GB="${ONCHAINAI_MIN_FREE_GB:-25}"
MAX_TARGET_GB="${ONCHAINAI_MAX_TARGET_GB:-35}"

free_kb="$(df -Pk . | awk 'NR==2 {print $4}')"
free_gb="$((free_kb / 1024 / 1024))"
target_gb="0"
if [[ -d target ]]; then
  target_kb="$(du -sk target | awk '{print $1}')"
  target_gb="$((target_kb / 1024 / 1024))"
fi

echo "free_disk_gb=${free_gb}"
echo "target_gb=${target_gb}"
du -sh target target/site .playwright-cli 2>/dev/null || true

if [[ "${ONCHAINAI_DISK_GUARD_FORCE:-0}" == "1" ]]; then
  echo "ONCHAINAI_DISK_GUARD_FORCE=1 set; continuing"
  exit 0
fi

if (( free_gb < MIN_FREE_GB )); then
  echo "ERROR: free disk ${free_gb}GB is below ${MIN_FREE_GB}GB" >&2
  echo "Run: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  exit 1
fi

if (( target_gb > MAX_TARGET_GB )); then
  echo "ERROR: target ${target_gb}GB exceeds ${MAX_TARGET_GB}GB" >&2
  echo "Run: ./scripts/clean-build-artifacts.sh --dry-run" >&2
  exit 1
fi
```

- [ ] **Step 2: Create `scripts/clean-build-artifacts.sh`**

Required options:

- `--dry-run`
- `--playwright-days N`
- no symlink traversal
- never delete `.env`, migrations, source, docs, or checked-in assets

Minimal implementation:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DRY_RUN=false
PLAYWRIGHT_DAYS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --playwright-days) PLAYWRIGHT_DAYS="${2:?missing days}"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

run() {
  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

if [[ -L target ]]; then
  echo "ERROR: target is a symlink; refusing cleanup" >&2
  exit 1
fi

if [[ -d target ]]; then
  run cargo clean
fi

if [[ -n "$PLAYWRIGHT_DAYS" && -d .playwright-cli && ! -L .playwright-cli ]]; then
  if [[ "$DRY_RUN" == true ]]; then
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -print
  else
    find .playwright-cli -type f -mtime "+${PLAYWRIGHT_DAYS}" -delete
  fi
fi
```

- [ ] **Step 3: Make scripts executable**

Run:

```bash
chmod +x scripts/disk-guard.sh scripts/clean-build-artifacts.sh
```

- [ ] **Step 4: Verify**

Run:

```bash
./scripts/disk-guard.sh
./scripts/clean-build-artifacts.sh --dry-run --playwright-days 7
```

Expected:

- Guard prints free disk and target size.
- Cleanup dry run does not delete files.

### Task 1.4: Add smoke tests

**Files:**
- Create: `scripts/smoke-test.sh`
- Optional create: `scripts/browser-smoke.mjs`
- Modify: `scripts/deploy-railway.sh`

- [ ] **Step 1: Create curl smoke**

Create `scripts/smoke-test.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE="${1:-http://localhost:3000}"
BASE="${BASE%/}"

fail() {
  echo "SMOKE FAIL: $*" >&2
  exit 1
}

check_get() {
  local path="$1"
  local body
  body="$(mktemp)"
  code="$(curl -sS -L -o "$body" -w "%{http_code}" "${BASE}${path}")" || fail "GET ${path} curl failed"
  [[ "$code" == "200" ]] || fail "GET ${path} returned ${code}"
  if grep -qiE "error deserializing|missing field filters|panic|not found: /pkg" "$body"; then
    echo "---- body excerpt ----" >&2
    head -80 "$body" >&2
    fail "GET ${path} contains app error"
  fi
}

check_get "/"
check_get "/tools"
check_get "/tools?function=bridge&type=mcp"

mcp_body="$(mktemp)"
mcp_code="$(curl -sS -o "$mcp_body" -w "%{http_code}" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "${BASE}/mcp")" || fail "POST /mcp curl failed"
[[ "$mcp_code" == "200" ]] || fail "POST /mcp returned ${mcp_code}"
grep -q '"serverInfo"' "$mcp_body" || fail "POST /mcp missing serverInfo"

echo "SMOKE PASS ${BASE}"
```

- [ ] **Step 2: Create browser smoke if Node/Playwright is available**

Create `scripts/browser-smoke.mjs`:

```javascript
import { chromium } from "playwright";

const base = (process.argv[2] || "http://localhost:3000").replace(/\/$/, "");
const errors = [];
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

page.on("console", (msg) => {
  if (["error", "warning"].includes(msg.type())) errors.push(`console:${msg.type()}:${msg.text()}`);
});
page.on("requestfailed", (req) => {
  errors.push(`requestfailed:${req.url()}:${req.failure()?.errorText}`);
});
page.on("response", async (res) => {
  const url = res.url();
  if (url.includes("/api") || url.includes("/pkg/")) {
    if (res.status() >= 400) errors.push(`http:${res.status()}:${url}`);
    const text = await res.text().catch(() => "");
    if (/error deserializing|missing field filters/i.test(text)) {
      errors.push(`body-error:${url}:${text.slice(0, 200)}`);
    }
  }
});

for (const path of ["/", "/tools", "/tools?function=bridge&type=mcp"]) {
  await page.goto(`${base}${path}`, { waitUntil: "networkidle" });
  const text = await page.textContent("body");
  if (/error deserializing|missing field filters/i.test(text || "")) {
    errors.push(`visible-error:${path}`);
  }
}

await browser.close();

if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`BROWSER SMOKE PASS ${base}`);
```

- [ ] **Step 3: Wire deploy script**

In `scripts/deploy-railway.sh`, after `railway up -y --detach`, add a smoke step that runs:

```bash
./scripts/smoke-test.sh "https://www.onchain-ai.xyz"
```

If Railway requires waiting, poll deployment status before smoke. The deploy script must exit non-zero when smoke fails.

- [ ] **Step 4: Verify locally**

Run server:

```bash
SKIP_CRAWLER=1 cargo run --features ssr
```

In another terminal:

```bash
./scripts/smoke-test.sh http://localhost:3000
node scripts/browser-smoke.mjs http://localhost:3000
```

Expected:

- Both smoke checks pass before deploy is considered successful.

### Task 1.5: Establish CSS source of truth

**Files:**
- Modify: `style/output.css`
- Modify: `src/lib.rs`
- Optional create: `style/input.css`
- Optional create: `tailwind.config.js`
- Optional create: `package.json`

- [ ] **Step 1: Decide CSS strategy**

Choose one strategy and record it in final summary:

- Strategy A: continue plain CSS and stop pretending it is generated Tailwind.
- Strategy B: add real Tailwind pipeline and make `style/output.css` generated.

Recommended for speed: Strategy A now, Tailwind pipeline later.

- [ ] **Step 2: For Strategy A, rename intent in CSS header**

Replace header in `style/output.css`:

```css
/* OnchainAI generated CSS — Tailwind output stub */
/* Replace with full Tailwind build when tooling is wired. */
```

With:

```css
/* OnchainAI stylesheet.
   This file is the current source of truth for production CSS.
   Do not rely on Tailwind utility classes unless they are defined here. */
```

- [ ] **Step 3: Add missing utilities used by current Rust**

Search:

```bash
rg -o 'class="[^"]+"' src | sed 's/.*class="//; s/".*//' | tr ' ' '\n' | sort -u
```

For each class used in Rust but not defined in `style/output.css`, either:

- add CSS, or
- replace the Rust class with an existing semantic class.

Known missing examples to handle:

- `md:text-[36px]`
- `md:text-[16px]`
- `border-t`
- `md:px-6`
- `md:py-12`

- [ ] **Step 4: Add UI smoke computed-style checks**

Extend browser smoke to assert:

- Home H1 font-size changes across desktop/mobile.
- `/pkg/onchainai.css` is non-empty.
- No visible `error deserializing`.
- Mobile chain overflow `+N` remains visible.
- Sidebar localStorage is cleared before screenshots:

```javascript
await page.evaluate(() => {
  localStorage.removeItem("onchain-ai-sidebar-collapsed");
  localStorage.removeItem("onchain-ai-sidebar-sections");
});
```

---

## Phase 2: Public Visibility Invariant

This phase blocks unsafe/off-topic listings at every surface.

### Task 2.1: Add public visibility rule

**Files:**
- Modify: `src/server/queries.rs`
- Modify: all public tool queries in `src/server/functions.rs`
- Modify: `src/server/mcp.rs`
- Add migration: `migrations/006_operator_hardening.sql`

- [ ] **Step 1: Add invariant to server query constants**

Replace:

```rust
pub const TOOLS_APPROVED_WHERE: &str = "approval_status = 'approved'";
```

With:

```rust
pub const PUBLIC_TOOL_WHERE: &str = "\
approval_status = 'approved' \
AND relevance_status = 'accepted' \
AND install_risk_level <> 'critical' \
AND quarantined_at IS NULL";
```

Keep this alias temporarily if needed:

```rust
pub const TOOLS_APPROVED_WHERE: &str = PUBLIC_TOOL_WHERE;
```

- [ ] **Step 2: Add migration columns**

Create `migrations/006_operator_hardening.sql` with:

```sql
ALTER TABLE tools
  ALTER COLUMN approval_status SET DEFAULT 'pending',
  ADD COLUMN IF NOT EXISTS crypto_relevance_score INT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS crypto_relevance_reasons TEXT[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS relevance_status TEXT NOT NULL DEFAULT 'needs_review',
  ADD COLUMN IF NOT EXISTS install_risk_level TEXT NOT NULL DEFAULT 'medium',
  ADD COLUMN IF NOT EXISTS install_risk_reasons TEXT[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS requires_secret BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS safe_copy_command TEXT,
  ADD COLUMN IF NOT EXISTS quarantined_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS last_reviewed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS review_policy_version TEXT NOT NULL DEFAULT 'operator-hardening-v1';

ALTER TABLE tools
  ADD CONSTRAINT tools_relevance_score_range
  CHECK (crypto_relevance_score >= 0 AND crypto_relevance_score <= 100);

ALTER TABLE tools
  ADD CONSTRAINT tools_relevance_status_check
  CHECK (relevance_status IN ('accepted', 'needs_review', 'rejected'));

ALTER TABLE tools
  ADD CONSTRAINT tools_install_risk_level_check
  CHECK (install_risk_level IN ('low', 'medium', 'high', 'critical'));
```

If constraints may already exist, wrap them in `DO $$ BEGIN IF NOT EXISTS ... END $$;`.

- [ ] **Step 3: Backfill safely**

Backfill existing rows conservatively:

```sql
UPDATE tools
SET relevance_status = 'needs_review',
    install_risk_level = CASE
      WHEN install_command IS NULL OR trim(install_command) = '' THEN 'medium'
      ELSE 'medium'
    END
WHERE relevance_status = 'needs_review';
```

Do not mass-mark existing rows `accepted` without scanner results. If production cannot tolerate hiding all rows, implement a temporary admin-reviewed allowlist and document it.

- [ ] **Step 4: Update RLS**

Drop public read policy and recreate:

```sql
DROP POLICY IF EXISTS "Public read tools" ON tools;

CREATE POLICY "Public read published tools" ON tools
  FOR SELECT TO anon, authenticated
  USING (
    approval_status = 'approved'
    AND relevance_status = 'accepted'
    AND install_risk_level <> 'critical'
    AND quarantined_at IS NULL
  );
```

Admin/server service-role access remains server-side.

- [ ] **Step 5: Verify public queries**

Run:

```bash
cargo test --features ssr public_tool_where
```

Expected:

- Tests assert `PUBLIC_TOOL_WHERE` contains approval, relevance, risk, and quarantine conditions.

### Task 2.2: Replace direct approval mutation with gated review

**Files:**
- Modify: `src/server/functions.rs`
- Modify: `src/pages/admin/tools.rs`
- Add model/table in migration for `tool_review_events`

- [ ] **Step 1: Add review events table**

Add to migration:

```sql
CREATE TABLE IF NOT EXISTS tool_review_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  admin_id UUID REFERENCES profiles(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  reason TEXT NOT NULL,
  override_reason TEXT,
  before_status TEXT,
  after_status TEXT,
  snapshot_id UUID,
  recommendation_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE tool_review_events ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Admin read review events" ON tool_review_events
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );
```

- [ ] **Step 2: Add review payload**

Add:

```rust
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
```

- [ ] **Step 3: Add server-side gate**

Approval must fail unless:

- `relevance_status == "accepted"`
- `install_risk_level != "critical"`
- at least one trustworthy URL/package/endpoint exists
- if overriding risk/relevance, `override_reason` is non-empty

Critical rule:

```rust
let override_required =
    tool.relevance_status == "rejected" || tool.install_risk_level == "critical";
if payload.action == "approved" && override_required && payload.override_reason.as_deref().unwrap_or("").trim().is_empty() {
    return Err(ServerFnError::new("override reason required"));
}
```

- [ ] **Step 4: Write audit event in same transaction**

Use one transaction:

1. require admin
2. re-read tool by slug
3. verify expected `updated_at` if present
4. enforce gate
5. insert `tool_review_events`
6. update `tools`
7. commit

- [ ] **Step 5: Keep old function as wrapper or remove after UI migration**

If cached clients/admin UI still call `set_tool_approval`, make it call the new function with a clear generic reason. Prefer migrating UI immediately.

---

## Phase 3: Relevance and Install Safety

### Task 3.1: Crypto relevance scanner

**Files:**
- Create: `src/crawler/relevance.rs`
- Modify: `src/crawler/mod.rs`
- Modify: `src/crawler/normalizer.rs`

- [ ] **Step 1: Add scanner output**

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RelevanceAssessment {
    pub score: i32,
    pub status: String,
    pub reasons: Vec<String>,
    pub negative_signals: Vec<String>,
}
```

- [ ] **Step 2: Add adversarial scoring rules**

Rules:

- Generic `mcp`, `sdk`, `agent`, `gateway`, `api` alone must not pass.
- Strong signals include named chains, DeFi protocols, wallet/custody, x402, RWA, contract tooling, crypto MCP topics.
- Keyword stuffing with no repo/homepage/npm crypto evidence should be `needs_review`.
- Weather/calendar/filesystem/productivity MCP should be `rejected`.

- [ ] **Step 3: Add tests**

Tests:

- accepted: BOB Gateway CLI with Bitcoin/EVM bridge terms.
- accepted: wallet MCP with chain support.
- needs_review: generic `web3 helper` with sparse evidence.
- rejected: filesystem MCP with `mcp` only.
- rejected: weather/calendar MCP.
- rejected or needs_review: suspicious keyword stuffing.

### Task 3.2: Install safety scanner

**Files:**
- Create: `src/install_safety.rs`
- Modify: `src/components/tool_detail_content.rs`
- Modify: `src/server/mcp.rs`

- [ ] **Step 1: Add scanner output**

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InstallSafetyAssessment {
    pub risk_level: String,
    pub reasons: Vec<String>,
    pub requires_secret: bool,
    pub safe_copy_command: Option<String>,
}
```

- [ ] **Step 2: Risk rules**

Classify as critical:

- `rm -rf`
- credential exfiltration patterns
- obfuscated `base64 -d | sh`
- command substitution that fetches remote code

Classify as high:

- `curl ... | sh`
- `wget ... | bash`
- `bash -c`
- `sh -c`
- shell redirection into config files

Classify as medium:

- requires API key/env vars
- global install
- unknown binary command

Classify as low:

- `npm i package`, `pnpm add package`, `cargo install package`, or documented package manager command with package metadata.

- [ ] **Step 3: Stop raw `sh -c` config generation**

Replace `claude_config(install: &str)` with structured rendering. If only legacy raw command exists and risk is high/critical, show warning and do not generate JSON config.

- [ ] **Step 4: MCP install guide includes warnings**

`get_install_guide` response should include:

- command
- risk level
- risk reasons
- warning text for medium/high/critical

Critical tools should return guidance that install is blocked pending review.

---

## Phase 4: Operator Console

**Files:**
- Modify: `src/pages/admin/mod.rs`
- Modify: `src/pages/admin/tools.rs`
- Modify: `src/pages/admin/crawler.rs`
- Modify: `src/server/functions.rs`

- [ ] **Step 1: Admin dashboard cards**

Admin home must show:

- pending candidates
- known updates
- high risk installs
- open reports
- crawler source health
- public tool count
- last successful crawl per source

- [ ] **Step 2: Review queue split**

Queues:

- `new_candidate`
- `known_update`
- `needs_manual_research`
- `low_relevance`
- `reported`
- `high_risk_install`

- [ ] **Step 3: Review row data**

Each row shows:

- name, slug, source
- repo/homepage/npm/MCP endpoint
- relevance score/status/reasons
- install risk/reasons
- duplicate candidates
- stars/last commit
- current lifecycle and claim state

- [ ] **Step 4: Review actions**

Actions:

- approve
- reject
- needs info
- quarantine
- mark verified
- mark official

All actions call gated server functions and write audit events.

---

## Phase 5: Submission, Updates, Reports, Claim Flow

**Files:**
- Create: `src/pages/submit.rs`
- Modify: `src/pages/mod.rs`
- Modify: `src/app.rs`
- Modify: `src/server/functions.rs`
- Add migration tables

- [ ] **Step 1: Add submission table**

Use a review-only table instead of direct public `tools` insert:

```sql
CREATE TABLE IF NOT EXISTS tool_submissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  submitted_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  payload JSONB NOT NULL,
  crypto_relevance_score INT NOT NULL DEFAULT 0,
  relevance_status TEXT NOT NULL DEFAULT 'needs_review',
  install_risk_level TEXT NOT NULL DEFAULT 'medium',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- [ ] **Step 2: Intake rule**

Submission may enter queue if minimally plausible. Public approval is where strict relevance/safety gates apply.

Replace any requirement like:

```text
Tool must pass minimum crypto relevance before it can be submitted.
```

With:

```text
Tool can be submitted if minimally plausible; crypto relevance gates public approval, not intake. Low-confidence submissions enter needs_manual_research.
```

- [ ] **Step 3: Add `/submit` route**

Add `SubmitPage` to `src/pages/mod.rs` and route in `src/app.rs`:

```rust
<Route path=StaticSegment("submit") view=SubmitPage/>
```

- [ ] **Step 4: Add report listing table and UI**

Report reasons:

- scam/phishing
- unsafe install
- wrong category
- not crypto-related
- broken link
- duplicate listing

- [ ] **Step 5: Add claim states**

Claim states:

- `unclaimed`
- `claim_pending`
- `claimed`
- `disputed`
- `revoked`

Official badge is separate from claim.

---

## Phase 6: Public UX and MCP Consistency

**Files:**
- Modify: `src/components/promo_cards.rs`
- Modify: `src/components/tool_detail_content.rs`
- Modify: `src/components/empty_state.rs`
- Modify: `src/server/mcp.rs`
- Modify: `style/output.css`

- [ ] **Step 1: Fix homepage promise**

Before self-service submission is fully shipped, copy should say:

```text
Suggest a Tool
Know a crypto MCP, CLI, SDK, API, or x402 tool we should review? Send it for operator review before it appears publicly.
```

Remove x402 paid registration copy until payments exist.

- [ ] **Step 2: Add trust panel**

Tool detail trust panel shows:

- source
- last crawl
- last commit
- relevance status
- install risk
- verification evidence
- report listing link

- [ ] **Step 3: Fix empty state**

Empty state includes:

- current filters
- clear filters
- suggest/submit tool CTA

- [ ] **Step 4: MCP output respects public invariant**

MCP tools must use `PUBLIC_TOOL_WHERE`, not only `approval_status`.

---

## Phase 7: Hermes Harness

**Files:**
- Create: `src/server/operator_harness.rs`
- Modify: `src/server/mod.rs`
- Modify: `src/lib.rs`
- Add migration tables: `operator_tasks`, `agent_action_proposals`

- [ ] **Step 1: Add bounded snapshot endpoint**

Route:

```text
GET /api/admin/operator/snapshot?queue=pending&limit=25
```

Response shape:

```rust
pub struct OperatorSnapshotV1 {
    pub snapshot_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub schema_version: String,
    pub limits: SnapshotLimits,
    pub truncated: bool,
    pub queue_summary: QueueSummary,
    pub tools: Vec<OperatorToolSnapshotV1>,
}
```

Hard limits:

- max 25 tools
- max 3 duplicate candidates per tool
- max 5 evidence snippets per tool
- max 500 chars per snippet
- no secrets
- no raw README dump

- [ ] **Step 2: Add recommendation endpoint**

Route:

```text
POST /api/admin/operator/run
```

It is read-only and returns `OperatorRecommendationV1[]`. It never mutates `tools`.

- [ ] **Step 3: Add action proposals table**

```sql
CREATE TABLE IF NOT EXISTS agent_action_proposals (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_name TEXT NOT NULL,
  action_type TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'proposed',
  subject_tool_id UUID REFERENCES tools(id) ON DELETE SET NULL,
  proposal JSONB NOT NULL,
  evidence JSONB NOT NULL DEFAULT '[]',
  approved_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  executed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- [ ] **Step 4: Enforce Hermes boundary**

Hermes can propose:

- approve
- reject
- needs info
- quarantine
- outreach

Hermes cannot directly execute:

- public approval
- official/verified badge
- deploy
- cleanup deletion
- auth/RLS/security changes

---

## Phase 8: Abuse, Rate, and Secret Safety

**Files:**
- Modify: `src/lib.rs`
- Modify: auth/server function modules
- Add tests

- [ ] **Step 1: Separate rate limit policy**

Add policy targets:

- submit: 5/hour/user
- comment: 10/min/user
- bookmark: 60/min/user
- MCP: separate IP limit
- auth: strict existing policy

- [ ] **Step 2: Secret redaction tests**

Harness and admin APIs must not return:

- `SUPABASE_SERVICE_KEY`
- `JWT_SECRET`
- `GITHUB_CLIENT_SECRET`
- full `.env` values

- [ ] **Step 3: Prompt-injection evidence safety**

Any README/package snippets shown to Hermes must be treated as untrusted evidence and bounded by length/hash/source URL.

---

## Phase 9: Final Verification and Deploy Gate

**Files:**
- All changed files
- `scripts/smoke-test.sh`
- `scripts/browser-smoke.mjs`

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 2: Tests**

Run:

```bash
cargo test --features ssr
```

Expected: PASS.

- [ ] **Step 3: Clippy**

Run:

```bash
cargo clippy --features ssr -- -W clippy::all
```

Expected: PASS.

- [ ] **Step 4: Disk guard**

Run:

```bash
./scripts/disk-guard.sh
```

Expected: PASS before heavy build.

- [ ] **Step 5: Full Leptos build**

Run:

```bash
cargo leptos build --release
```

Expected:

- `target/release/onchainai` exists and is non-empty.
- `target/site/pkg/onchainai.js` exists and is non-empty.
- `target/site/pkg/onchainai.wasm` exists and is non-empty.
- CSS served in production path is non-empty.

- [ ] **Step 6: Local smoke**

Run:

```bash
SKIP_CRAWLER=1 cargo run --features ssr
./scripts/smoke-test.sh http://localhost:3000
node scripts/browser-smoke.mjs http://localhost:3000
```

Expected:

- Both smoke tests pass.
- No `error deserializing`.
- No `missing field filters`.
- `/mcp` initialize returns serverInfo.

- [ ] **Step 7: Production deploy**

Run only after local gates pass:

```bash
./scripts/deploy-railway.sh
```

Expected:

- Deploy script waits for deployment.
- Production smoke passes:

```bash
./scripts/smoke-test.sh https://www.onchain-ai.xyz
node scripts/browser-smoke.mjs https://www.onchain-ai.xyz
```

No release is complete until production smoke passes.

---

## Merge/Deploy Risk Gates

Do not merge or deploy if any gate fails:

- Production Docker build can skip WASM/JS bundle failure.
- `/tools` or filtered `/tools` shows server-function argument errors.
- `/pkg/onchainai.js` or `/pkg/onchainai.wasm` is missing or stale.
- CSS route serves 0 bytes.
- Public `tools` visibility is only `approval_status='approved'`.
- RLS allows anon users to read pending/rejected/critical/quarantined tools.
- Critical install risk can be approved without override reason.
- Approval does not write audit event.
- `/submit` creates public tools directly.
- Hermes endpoint can mutate public listing state directly.
- Smoke tests are not run after deploy.

