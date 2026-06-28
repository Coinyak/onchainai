# Public Dashboard And My Toolkit Design

> Related docs: [[../../UI_UX_DESIGN]] | [[../../../DESIGN]] | [[../../BUILD_DEPLOY_RULES]] | [[../../SECURITY]] | [[../../X402_REFERRAL_SPEC]] | [[../../../AGENTS.md]]
>
> Date: 2026-06-28
> Status: Design spec ready for user review
> Scope: Public login-free dashboard, authenticated My Toolkit, shared export model, and optional public MCP dashboard snapshot

---

## 0. Direction

OnchainAI should build both features, but they should have different jobs:

- **Public Dashboard**: a login-free ecosystem dashboard that helps any visitor or agent understand the public crypto tool landscape.
- **My Toolkit**: an authenticated saved-tool workspace that lets a user collect tools and export agent-ready install/config bundles.

The product positioning becomes:

> OnchainAI is a crypto tool directory and agent toolkit builder.

The dashboard is the public market map. My Toolkit is the personal assembly surface.

---

## 1. Benchmark Review

Benchmarked on 2026-06-28. These products prove that the category exists, but none matches OnchainAI's full wedge of crypto-specific MCP/CLI/SDK/API/x402/RWA/agent tooling plus public trust signals plus personal toolkit export.

| Product | What it offers | What to learn | Gap OnchainAI can own |
|---|---|---|---|
| [Smithery](https://smithery.ai/) | MCP tools/skills discovery with CLI search, managed MCP connections, namespaces, and service-token oriented usage. | The managed connection/toolbox mental model is clear and should influence My Toolkit naming. | Broad MCP registry, not crypto-specific, not focused on x402/RWA/agent commerce trust. |
| [Glama MCP](https://glama.ai/mcp) | MCP search, comparison, scans, quality/security/ease-of-use ranking, browser testing/sandbox positioning. | Dashboard should expose quality and compatibility signals, not just raw counts. | Strong MCP focus, but less crypto payment/toolchain-specific context. |
| [PulseMCP](https://www.pulsemcp.com/servers) | Daily updated MCP directory with 20,000+ servers, classification, estimated weekly visitors, release dates, and ecosystem content. | Public freshness, trend, and release metadata make a directory feel alive. | Does not solve crypto-specific trust, x402, chain, and agent payment context. |
| [mcp.so](https://mcp.so/) | Community MCP server directory with search, categories, tags, clients, feed, and submit flow. | SEO-friendly category pages and broad browse paths matter. | Community breadth can reduce trust; OnchainAI should be more curated and evidence-backed. |
| [Official MCP Registry](https://modelcontextprotocol.io/registry/about) | Official metadata repository, REST API, DNS/GitHub namespace verification, standardized install metadata. | Separate unopinionated source metadata from curated directory judgments. | It is intentionally not the end-user marketplace/dashboard layer. |
| [Agentic.Market](https://www.coinbase.com/developer-platform/discover/launches/agentic-market) | Public x402 marketplace for discovering, comparing, and integrating services with live pricing, volume data, top lists, and integration guides, without required login. | This validates login-free x402 service discovery, machine-readable market views, and data-rich public dashboards. | OnchainAI can be broader than x402-only and add crypto tool installation/trust/export. |
| [x402 Bazaar](https://docs.cdp.coinbase.com/x402/bazaar) | Discovery layer for browsing/searching x402-enabled payable API endpoints with semantic descriptions, payment metadata, and trust signals. | Agents want discovery through MCP/API-style surfaces, not only a human web UI. | OnchainAI should expose discovery metadata, but not become a payment/custody/facilitator proxy. |
| [Composio Toolkits](https://composio.dev/toolkits) | Large integration catalog with MCP/direct API access, managed auth, delegated access, and multi-app workflows. | Toolkit export should target real agent clients and workflows, not just saved links. | Generic SaaS/workflow integrations, not crypto-native discovery or x402 trust. |

### Benchmark Conclusion

There are already public MCP directories, x402 service discovery layers, and managed agent-integration catalogs. The defensible OnchainAI angle is the combination:

1. crypto-specific normalization across MCP, CLI, SDK, API, x402, RWA, and AI-agent tools
2. login-free public dashboard for ecosystem visibility
3. evidence-backed trust and x402 verification labels
4. authenticated toolkit builder that exports practical agent install/config bundles
5. public MCP endpoint for agents to search the same approved catalog

---

## 2. Product Split

### 2.1 Public Dashboard

Route: `/dashboard`

Access: public, no login required.

Primary user tasks:

- understand the crypto tool ecosystem at a glance
- filter into the normal `/tools` browser by type, function, chain, pricing, and trust status
- discover new, popular, x402, verified, and official tools
- decide whether to save a tool into My Toolkit
- let agents retrieve the same public snapshot through MCP

Non-goals:

- no user-specific data
- no admin review queues
- no pending/rejected/quarantined tools
- no hidden payout addresses or provider payment addresses
- no x402 payment execution
- no wallet connection requirement

### 2.2 My Toolkit

Route: `/toolkit`

Access: authenticated.

Primary user tasks:

- view saved tools from the existing bookmarks table
- remove tools from the saved list
- copy individual install commands
- export the whole toolkit as an agent-ready bundle
- jump back to `/tools` or `/dashboard` to add more tools

Non-goals for v1:

- no checkout/cart purchase flow
- no multi-user teams
- no named collections
- no automatic tool execution
- no x402 payment signing through OnchainAI

### 2.3 Naming Decision

Use **Toolkit**, not "cart".

Reason:

- "Cart" implies purchase/checkout.
- x402 and referral rules explicitly avoid custody, facilitator, and fund-moving paths.
- "Toolkit" matches the user's mental model: collect the products/tools needed for an agent setup, then call or install them in the user's own environment.

UI labels should be English:

- `Dashboard`
- `My Toolkit`
- `Save to Toolkit`
- `Saved`
- `Export`
- `Copy config`
- `Open install guide`

---

## 3. Public Dashboard Specification

### 3.1 First Viewport

The first screen must show the actual dashboard, not a marketing landing page.

Desktop order:

1. Sidebar brand/nav.
2. Page title: `Dashboard`.
3. Short subtitle: `Public snapshot of approved crypto tools across MCP, CLI, SDK, API, x402, RWA, and agent workflows.`
4. Metric strip.
5. Two-column dashboard body:
   - left/wide: ecosystem distribution and trend/list modules
   - right/narrow: x402/trust/freshness modules

Mobile order:

1. Sidebar rail or collapsed sidebar behavior follows the existing site pattern.
2. Title/subtitle.
3. Horizontally scrollable metric strip or two-column compact metric grid.
4. Stacked modules with no horizontal overflow.

No hero-scale marketing composition. No gradient panels. No decorative crypto visual noise.

### 3.2 Metric Strip

Cards:

- `Public tools`
- `MCP`
- `CLI`
- `SDK`
- `API`
- `x402`
- `Official`
- `Verified`
- `Updated recently`

Each metric card:

- 8px radius max
- neutral border
- label in small caps
- value in compact dashboard-scale type, not hero type
- optional small secondary line, e.g. `approved only`

Click behavior:

- `MCP` -> `/tools?type=mcp`
- `CLI` -> `/tools?type=cli`
- `SDK` -> `/tools?type=sdk`
- `API` -> `/tools?type=api`
- `x402` -> `/tools?type=x402`
- `Official` -> `/tools?status=official`
- `Verified` -> `/tools?status=verified`

### 3.3 Distribution Modules

Use simple native HTML/CSS bar lists first. Avoid introducing a chart library for v1 unless implementation later proves it is already present.

Modules:

- `By type`: MCP, CLI, SDK, API, Skill, x402.
- `By function`: top categories such as Bridge, Data, Payments, Wallet, Trading, AI Agent.
- `By chain`: top chain values from the chain catalog.
- `By trust`: Official, Verified, Community.
- `By pricing`: Free, Freemium, Paid, x402, Unknown.

Every segment links to the matching `/tools` filter where possible.

### 3.4 Live Lists

Dashboard lists should be short, scannable, and linked to existing detail/preview surfaces.

Sections:

- `Newly listed`: newest approved tools.
- `Popular`: highest GitHub stars and/or comments.
- `x402 tools`: approved tools with `pricing = 'x402'` or `x402_price IS NOT NULL`.
- `High-trust tools`: official or verified tools with low install risk.
- `Needs inspection`: public but unverified x402 payment details or community tools with meaningful adoption. This is not an admin queue; it is a public "inspect before use" education list.

Each row:

- logo/monogram
- name
- badges: type, trust, x402 if applicable
- short description, 1-2 lines
- primary link to detail
- secondary `Save to Toolkit` action

Anonymous `Save to Toolkit` behavior:

- show login modal
- message: `Sign in to save tools to My Toolkit.`
- after login, the server-side bookmark flow persists the tool

### 3.5 Freshness And Source Context

Show a small `Data freshness` module:

- `Snapshot as of <timestamp>`
- `Latest public listing update`
- `Crawler sources active` count if available from public-safe source status

Do not show:

- failed internal jobs with stack traces
- admin-only crawler controls
- pending review counts
- service secrets or exact operator diagnostics

### 3.6 Empty And Low-Data States

If the catalog is small:

- keep the dashboard visible
- show zeros honestly
- link to `/submit`
- do not fake trend lines
- do not hide modules that make the product understandable

Copy:

- `No public x402 tools yet.`
- `No verified tools in this slice yet.`
- `Submit a tool for operator review.`

---

## 4. My Toolkit Specification

### 4.1 Data Model

V1 reuses existing bookmarks:

- `bookmarks.tool_id`
- `bookmarks.user_id`
- `bookmarks.created_at`
- unique `(tool_id, user_id)`

No new database tables are required for the first version.

Future extension, not v1:

- `toolkit_collections`
- `toolkit_items`
- `toolkit_export_events`
- shared public toolkit links

### 4.2 Toolkit Page

Route: `/toolkit`

Authenticated page content:

- title: `My Toolkit`
- subtitle: `Saved crypto tools and agent install bundles.`
- saved tool count
- saved tools grouped by type
- export panel
- empty state with links to `/dashboard` and `/tools`

Unauthenticated behavior:

- render sign-in prompt inside `SiteShell`
- do not server-render private placeholder data
- copy: `Sign in to view and export your saved toolkit.`

### 4.3 Saved Tool Rows

Each saved tool row:

- logo/monogram
- name
- type/trust/x402 badges
- chain tags
- description
- saved date
- `Open`
- `Copy install`
- `Remove`

Ordering:

1. newest saved first by default
2. optional grouped view by type
3. future sort by name, type, trust

### 4.4 Export Panel

Export formats:

- `Markdown`: human-readable list with links, descriptions, install commands, x402 notes.
- `Claude`: MCP config JSON where safe structured config exists.
- `Cursor`: install/config instructions where supported.
- `Generic`: compact JSON for agents or scripts.

Export content must include:

- tool slug
- tool name
- tool type
- short description
- install command or MCP endpoint when available
- official links
- trust labels
- x402 notice when relevant

Export content must not include:

- `referral_payout_address`
- `x402_pay_to_address`
- private user identifiers
- pending/rejected/quarantined tools
- unredacted secrets from install commands

### 4.5 Save Action Across The Site

The existing bookmark button should become product-language aligned:

- accessible label: `Save to Toolkit` or `Remove from Toolkit`
- visual state: saved vs unsaved
- anonymous click: login modal
- authenticated click: existing `toggle_bookmark` flow

Implementation can still use the `bookmarks` table internally.

---

## 5. MCP And Agent Surface

### 5.1 Existing MCP Tools

The current MCP server already supports public discovery:

- `search_tools`
- `get_tool_detail`
- `list_categories`
- `get_install_guide`

These tools should continue to use the same public quality filter as the web catalog.

### 5.2 Optional New Public MCP Tool

Add a public MCP tool only after the web snapshot server function exists:

Name: `get_dashboard_snapshot`

Purpose:

- return the same public aggregate snapshot that backs `/dashboard`
- let agents understand ecosystem distribution without scraping the website

Input:

```json
{
  "type": "object",
  "properties": {
    "limit": { "type": "integer", "minimum": 1, "maximum": 25 }
  }
}
```

Output:

- summary metrics
- top type/function/chain/trust/pricing buckets
- short public lists: new, popular, x402, high-trust
- `as_of` timestamp

Do not add personal toolkit MCP access in v1. The current `/mcp` endpoint is public and IP-rate-limited, not user-authenticated. Exposing private bookmarks through it would need a separate auth design.

### 5.3 x402 Boundary

OnchainAI may:

- expose x402 metadata
- show x402 price text
- show verification flags as trust signals
- record referral attribution if separately implemented
- link users/agents to provider install/call instructions

OnchainAI must not:

- hold user or provider funds
- proxy facilitator payments
- sign payments for users
- create undocumented `referrer` or `split` payment request fields
- hide otherwise approved public tools only because x402 verification flags are false

---

## 6. Server And Data Design

### 6.1 Public Dashboard Server Function

Add a public server function:

```rust
#[server(GetPublicDashboardSnapshot, "/api")]
pub async fn get_public_dashboard_snapshot() -> Result<PublicDashboardSnapshot, ServerFnError>
```

The function must query only rows matching `TOOLS_APPROVED_WHERE`.

Suggested payload:

```rust
pub struct PublicDashboardSnapshot {
    pub as_of: DateTime<Utc>,
    pub metrics: DashboardMetrics,
    pub type_counts: Vec<DashboardBucket>,
    pub function_counts: Vec<DashboardBucket>,
    pub chain_counts: Vec<DashboardBucket>,
    pub trust_counts: Vec<DashboardBucket>,
    pub pricing_counts: Vec<DashboardBucket>,
    pub new_tools: Vec<Tool>,
    pub popular_tools: Vec<Tool>,
    pub x402_tools: Vec<Tool>,
    pub high_trust_tools: Vec<Tool>,
}
```

Use sanitized public tool responses before returning any `Tool` payload.

### 6.2 Toolkit Server Functions

Add authenticated server functions:

```rust
#[server(ListMyToolkit, "/api")]
pub async fn list_my_toolkit() -> Result<MyToolkitPayload, ServerFnError>

#[server(GetMyToolkitExport, "/api")]
pub async fn get_my_toolkit_export(format: ToolkitExportFormat) -> Result<ToolkitExport, ServerFnError>
```

Both functions:

- call `require_user`
- join `bookmarks` to `tools`
- apply `TOOLS_APPROVED_WHERE`
- sanitize returned tool fields
- preserve stable ordering

Do not expose removed, quarantined, or no-longer-public tools in exports.

### 6.3 Query Performance

Keep the dashboard bounded:

- bucket counts limited to top 12 where applicable
- lists limited to 5-10 tools
- no unbounded JSON payloads
- no client fan-out into many `/api` calls

The dashboard should use one bundled server function call, similar to `LoadBrowserData`.

---

## 7. UI Architecture

### 7.1 New Files

Likely files:

- `src/pages/dashboard.rs`
- `src/pages/toolkit.rs`
- `src/components/dashboard_metric_card.rs`
- `src/components/dashboard_bar_list.rs`
- `src/components/dashboard_tool_list.rs`
- `src/components/toolkit_export_panel.rs`

Register routes in:

- `src/app.rs`
- `src/pages/mod.rs`
- `src/components/mod.rs` if shared components are added

### 7.2 Visual Rules

Follow `DESIGN.md`:

- light mode only
- neutral surfaces
- orange only for the primary action or active state
- 8px card radius max
- no gradients
- no emoji
- no decorative blobs/orbs
- mobile body text at least 16px
- touch targets at least 44px

Dashboard should feel like an operational product surface, not a SaaS marketing page.

### 7.3 Navigation

Add public nav affordances:

- Sidebar brand/nav should expose `Dashboard`.
- Authenticated users should see `My Toolkit`.
- Anonymous users can still see `My Toolkit` link if it opens the sign-in prompt page; this makes the feature discoverable.

### 7.4 Responsive Behavior

Desktop `1280x900`:

- metric strip remains readable
- two-column body has no card-in-card nesting
- right column modules do not squeeze text

Mobile `375x812`:

- all modules stack
- metric cards wrap into two columns or scroll horizontally with visible affordance
- no horizontal page scroll
- tool rows keep actions as icons/buttons with 44px targets
- export controls use segmented buttons or tabs, not oversized text cards

---

## 8. Security And Privacy

Public dashboard:

- only approved public tools
- only sanitized public fields
- no private user data
- no bookmark totals by user
- no admin queue counts
- no pending/rejected/quarantined listings
- no payout or pay-to addresses

My Toolkit:

- requires authenticated session
- uses existing bookmark RLS and server-side `require_user`
- never trusts client-provided user ids
- no public sharing in v1

x402:

- display verification flags as trust signals
- no payment execution UI
- no custody or facilitator proxy
- no hidden automatic split/referrer fields

---

## 9. Testing And Verification

Minimum implementation gates:

```bash
cargo test --features ssr
cargo clippy --features ssr -- -W clippy::all
cargo fmt --check
git diff --check
```

For UI implementation:

```bash
./scripts/disk-guard.sh
./scripts/release-build.sh
./scripts/verify-bundle.sh
./scripts/restart-dev.sh --no-build
./scripts/smoke-test.sh http://localhost:3000
node scripts/browser-smoke.mjs http://localhost:3000
node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots
```

Add or update smoke expectations:

- `GET /dashboard` returns 200 and contains dashboard route markup.
- `GET /dashboard` has no auth wall.
- `GET /toolkit` returns 200 with sign-in prompt when anonymous.
- public dashboard body does not contain payout address fields.
- dashboard links navigate into `/tools` filters.

Targeted tests:

- dashboard snapshot uses `TOOLS_APPROVED_WHERE`
- dashboard snapshot strips payout/pay-to addresses
- toolkit list requires auth
- toolkit export excludes non-public tools
- save/remove action keeps existing bookmark rate limit behavior
- MCP `get_dashboard_snapshot`, if added, uses the same public snapshot and rate limit path

Visual QA:

- inspect `/dashboard` at `1280x900`
- inspect `/dashboard` at `375x812`
- inspect `/toolkit` empty/auth prompt at `375x812`
- inspect `/toolkit` with seeded bookmarks at `1280x900`

---

## 10. Implementation Phases

### Phase 1: Public Dashboard Foundation

Goal: ship login-free `/dashboard`.

Includes:

- `PublicDashboardSnapshot`
- dashboard route
- metric strip
- distribution modules
- live lists
- links into `/tools`
- no new DB schema

Why first:

- proves public value before asking users to sign in
- creates SEO/discovery surface
- reuses existing public catalog data
- gives agents a stable snapshot model if MCP is added later

### Phase 2: My Toolkit V1

Goal: turn bookmarks into a coherent saved-tool workspace.

Includes:

- `/toolkit`
- authenticated saved list
- empty state
- remove action
- copy install
- export panel
- update bookmark wording to `Save to Toolkit`

Why second:

- builds on existing bookmarks
- converts discovery into retention
- turns OnchainAI into a practical agent setup assistant

### Phase 3: Dashboard Snapshot MCP Tool

Goal: expose the public dashboard snapshot to agents.

Includes:

- `get_dashboard_snapshot` MCP tool
- same server payload as web dashboard
- bounded list limits
- no personal toolkit data

Why third:

- avoids designing agent payload before the web snapshot stabilizes
- preserves privacy and auth boundaries

### Phase 4: Advanced Toolkit Collections

Future only:

- named collections
- shared public toolkit links
- team workspaces
- toolkit templates such as `Base data agent`, `x402 monitor`, `RWA analyst`

---

## 11. Acceptance Criteria

Public Dashboard is acceptable when:

- anonymous users can use `/dashboard` without login
- all data is public-approved and sanitized
- metric cards link to correct filtered `/tools` URLs
- x402 tools show trust/verification notes without payment execution
- mobile view has no horizontal overflow or clipped actions
- visual style matches `DESIGN.md`

My Toolkit is acceptable when:

- anonymous users get a clear sign-in prompt
- authenticated users see saved tools from bookmarks
- users can remove saved tools
- export formats work for Markdown and Generic JSON at minimum
- Claude/Cursor export only emits safe structured config when available
- exports exclude hidden/sensitive fields

MCP snapshot is acceptable when:

- agents can retrieve public dashboard metrics through `/mcp`
- result size is bounded
- no user-specific toolkit data is exposed
- the MCP response matches the web dashboard's public filtering rules

---

## 12. Spec Self-Review

Placeholder scan:

- No `TBD` or unresolved placeholders remain.

Internal consistency:

- `/dashboard` is public and uses only approved public data.
- `/toolkit` is authenticated and reuses bookmarks.
- MCP v1 remains public-only for dashboard snapshot, not private toolkit access.

Scope check:

- The design is split into implementation phases so the first build can stay narrow.
- No schema migration is required until future named collections.

Security check:

- x402 remains metadata/trust/referral context only.
- No custody, payment proxy, wallet signing, or undocumented payment-field design is introduced.

Ambiguity check:

- "Cart" is explicitly renamed to "Toolkit".
- Login-free requirement applies to `/dashboard`, not private saved-tool persistence.
