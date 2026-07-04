# User-Friendly Discovery & Decision UX — Implementation Spec

Status: draft
Date: 2026-07-01
Scope: public discovery, tool selection, trust explanation, safe install, saved toolkit

## 1. Summary

OnchainAI should help a visitor move from "I need an onchain tool" to "I trust
this tool enough to install or save it" with less manual filtering and less
guesswork.

Add six user-facing layers:

1. **Tool Finder** — a guided, question-style entry point that turns intent into
   filters and search terms.
2. **Compare Tools** — a lightweight comparison drawer/page for 2-3 selected
   tools.
3. **Why Trust This?** — an explainable trust box on tool detail/preview, focused
   on official links and evidence instead of opaque scores.
4. **Safe Install / Use With Agent** — platform-specific install/config actions
   for Claude, Cursor, generic MCP, and CLI/SDK copy flows.
5. **Saved Toolkit Improvements** — turn bookmarks into a personal working set
   with notes, tags, export, and compare.
6. **Search & Empty-State Improvements** — natural-language-ish search helpers
   and actionable no-results recovery.

All UI copy in the product remains English. This document is Korean/English mixed
for implementation clarity.

## 1.1 Current Codebase Facts

This spec builds on existing surfaces, not a blank slate:

- Public browsing already supports URL-backed filters for `function`,
  `asset_class`, `actor`, `type`, `status`, `pricing`, and `chain`.
- Public browsing does **not** yet expose an install-risk filter. Any finder,
  search helper, or empty-state suggestion that refers to risk must first add a
  public `install_risk` filter axis and wire it through `ToolFilters`, URL
  parsing/building, validation, and list SQL.
- Tool detail already renders trust facts, official links, install-risk copy,
  and Generic/Claude/Cursor install tabs. `Why trust this?` and `Safe install`
  should consolidate and improve those existing sections instead of creating
  duplicate panels.
- `/toolkit` already uses `bookmarks` as the saved-tool model and already has
  Markdown/JSON exports. Toolkit improvements should extend that model unless a
  later migration proves a separate metadata table is cleaner.

## 2. Goals

- Help non-expert users discover useful tools without understanding every
  sidebar filter.
- Make tool choice easier by comparing the dimensions that matter: type, chains,
  status, risk, official links, install path, and recent activity.
- Make trust explainable: show official GitHub, website, X, verification facts,
  and install risk. Keep raw trust score operator-facing.
- Make install safer and more practical for agent users.
- Give signed-in users a reason to return through a better saved toolkit.
- Preserve the existing information-dense, calm, light UI.

## 3. Non-Goals

- No wallet custody, payment execution, x402 facilitator, fund-moving, or
  transaction submission UI.
- No AI-generated tool claims unless backed by current database fields or
  server-side evidence.
- No public raw trust score.
- No forced onboarding before browsing.
- No replacing the existing sidebar, chain strip, filters, sorting, pagination,
  preview panel, or auth flow.
- No auto-triggered review bots or CI.

## 4. Target Users

- **Builder browsing quickly:** wants the best MCP/CLI/SDK for a chain/task.
- **Agent user:** wants a tool that can be pasted into Claude/Cursor/Codex-style
  workflows safely.
- **Cautious installer:** wants to know whether install commands and official
  links are trustworthy.
- **Returning user:** saves a stack of tools and exports them later.

## 5. Feature A — Tool Finder

### User Job

"I know the task I want, but I do not know which filters to use."

### Entry Points

- Home hero area, above or near the main search bar.
- Empty state recovery.
- Optional compact button on `/tools`: `Find a tool`.

### UX

Use a compact guided panel, not a marketing wizard. It should feel like a
filter assistant:

1. `What are you building?`
   - Wallet / Portfolio
   - Trading / Swap
   - Bridge
   - Data / Indexing
   - Payments / x402
   - AI Agent / MCP
2. `Where should it work?`
   - All chains
   - Bitcoin
   - Ethereum
   - Base
   - Solana
   - More chains
3. `How will you use it?`
   - MCP server
   - CLI
   - SDK
   - API
   - No preference
4. `Install safety`
   - Low risk only
   - Verified/official preferred
   - Show all except critical

The result navigates to existing public URLs, for example:

- `/tools?function=bridge&chain=base&type=mcp&install_risk=low`
- `/tools?q=portfolio%20mcp&chain=ethereum&type=mcp`

### Implementation Shape

- New component: `ToolFinderPanel`.
- Keep state client-side, backed by URL generation only.
- Reuse existing filter query helpers where possible.
- No server function required for v1.
- Add pure helper tests for mapping answers to query params.
- Add `install_risk` as a first-class public filter before shipping any
  `Low risk only` or `Show all except critical` finder path.

### Acceptance Criteria

- Works on desktop and mobile without horizontal scroll.
- Generated links preserve existing filter semantics.
- User can close or reset the panel.
- No result mutation; it only navigates to existing browse surfaces.

## 6. Feature B — Compare Tools

### User Job

"I found a few candidates. Which one should I choose?"

### Entry Points

- Tool cards: small checkbox/icon action `Compare`.
- Tool preview/detail: `Add to compare`.
- Toolkit page: compare saved tools.

### UX

When 1-3 tools are selected, show a sticky compare tray:

- `2 selected`
- `Compare`
- `Clear`

`Compare` navigates to the canonical route
`/compare?tools=slug-a,slug-b,slug-c`. A sticky tray or drawer can be used as
the selection affordance, but the route is the source of truth for shareability,
reloads, and tests.

Comparison dimensions:

- Name and logo
- Status: Community / Verified / Official
- Type: MCP / CLI / SDK / API / x402 / RWA / agent
- Supported chains
- Install risk
- Install command/config availability
- Official links: GitHub / website / X / docs / package
- Stars and last commit/update
- Claimed/team status
- Saved/bookmarked state

### Data

V1 can load comparison rows with existing public tool list/detail data if all
needed fields are already public. If missing, add:

- `compare_tools(slugs: Vec<String>) -> Vec<ToolComparisonView>`

Server-side rules:

- Max 3 slugs.
- Public visibility filters identical to public tool listing.
- No private/admin-only fields.
- Stable slug ordering matching the user selection.

### Acceptance Criteria

- At most 3 tools can be compared; the UI explains the limit.
- Compare selection persists in URL or session/local storage across route
  changes.
- Mobile uses stacked comparison sections, not a squeezed table.
- Empty/missing tool slugs are ignored with a soft message.

## 7. Feature C — Why Trust This?

### User Job

"Can I trust this tool, and why?"

### Placement

- Tool preview panel.
- Tool detail page.
- Compare view row/section.

### UX

Consolidate the existing `ToolTrustFacts`, `OfficialLinksList`, and
`Activity and safety` sections into a single clearer box titled
`Why trust this?`. Avoid rendering duplicate trust panels.

Recommended visible facts:

- `Official GitHub` link if verified/known.
- `Official website` link if verified/known.
- `X / social` link if verified/known.
- `Install risk`: Low / Medium / High / Critical.
- `Last checked` or `Last reviewed` if available.
- `Recent activity`: last commit/update.
- `Claimed by team` / `Official` status.
- `Evidence gaps`: short, human-readable list when important.

Do not show raw trust score publicly. If an internal score is useful, keep it in
admin/review workbench only.

### Data

Prefer existing structures:

- `ToolOfficialLink`
- trust facts from `trust_verification`
- tool status / claim state
- install risk fields

If the public view lacks enough evidence in a single payload, add:

```text
PublicTrustSummary {
  official_links: Vec<PublicOfficialLink>,
  trust_facts: Vec<String>,
  evidence_gaps: Vec<String>,
  install_risk_level: String,
  last_reviewed_at: Option<DateTime<Utc>>,
  last_commit_at: Option<DateTime<Utc>>,
  claim_state: String,
}
```

### Acceptance Criteria

- A public user can see official links without opening admin.
- Critical/high install risk is visible before copying an install command.
- Raw numeric trust score is absent from public UI.
- Links open safely with `rel="noopener noreferrer"` where applicable.

## 8. Feature D — Safe Install / Use With Agent

### User Job

"I chose a tool. How do I safely use it in my agent or terminal?"

### Placement

- Tool preview panel.
- Tool detail page install section.
- Compare view action row.

### UX

Extend the existing install tabs/actions into a safer platform flow:

- `Claude`
- `Cursor`
- `Generic MCP`
- `CLI / SDK`

Actions:

- `Copy config`
- `Copy command`
- `View safe install notes`
- `Open docs`

Risk behavior:

- `critical`: block copy/run affordances; show why it is blocked.
- `high`: show warning and require explicit reveal/copy confirmation.
- `medium`: show caution note.
- `low`: normal copy flow.

### Data/API

Reuse or extend current MCP install guide logic:

- Server helper should generate platform-specific config from stored fields.
- Never invent install commands.
- Never expose secrets or private payout/payment fields.

Possible server function:

```text
get_public_install_guide(slug, platform) -> PublicInstallGuide
```

Fields:

- `platform`
- `risk_level`
- `risk_notes`
- `copy_blocks`
- `docs_links`
- `blocked`

### Acceptance Criteria

- Copy buttons do not appear for critical-risk commands.
- Platform tabs are keyboard accessible.
- Copied text matches server-generated guide exactly.
- Mobile install section wraps cleanly and remains readable.

## 9. Feature E — Saved Toolkit Improvements

### User Job

"I want to save my tool stack and come back later."

### Current Baseline

There is already a signed-in toolkit/bookmark surface. Improve it without
breaking existing bookmark behavior or existing Markdown/JSON exports.

### UX

Enhance `/toolkit`:

- Saved tools grouped by optional user tags.
- User notes per saved tool.
- `Compare selected`.
- `Export toolkit` with selected formats:
  - Markdown
  - JSON
  - MCP config bundle where safe
- `Recently saved` / `Recently updated` sorting.

### Data

Use the existing `bookmarks` table as the base. Add optional metadata columns
unless a migration review shows that a separate metadata table is safer:

```text
bookmarks
  note text nullable
  tags text[] default '{}'
  updated_at timestamptz default now()
```

Keep the existing `(tool_id, user_id)` uniqueness and owner-only RLS policies.
If a separate table is chosen later, it must preserve the same auth boundary and
avoid splitting one saved-tool concept across two unrelated UI models.

### Acceptance Criteria

- Existing bookmarks continue to render.
- Notes/tags require auth and are server-side scoped to the current user.
- Export redacts unsafe/private fields and respects install risk blocking.
- Toolkit compare reuses Compare Tools logic.

## 10. Feature F — Search & Empty-State Improvements

### Search Helpers

Add lightweight query interpretation before deeper AI search:

Examples:

- `base wallet mcp` -> `q=wallet mcp&chain=base&type=mcp`
- `low risk x402` -> `type=x402&install_risk=low`
- `solana agent sdk` -> `chain=solana&function=ai-agent&type=sdk`

Implementation:

- Pure helper: `parse_search_intent(query) -> SearchIntent`.
- Only map high-confidence tokens.
- Preserve the raw `q` so the user can see/edit what they typed.
- Avoid hidden magic that removes user intent.

### Empty State

When no tools match, show:

- Current filters summary.
- `Clear filters`.
- Suggested one-click relaxations:
  - Remove chain filter
  - Show all types
  - Include medium risk
  - Search all tools for this keyword
- `Submit a missing tool`.

### Acceptance Criteria

- Empty state always offers at least one recovery action.
- Suggestions never add stricter filters than the current state.
- Existing clear-filter behavior remains.

## 11. Navigation & Information Architecture

Recommended additions:

- Home: `Tool Finder` near search.
- Tool card: `Compare` action, admin quick actions remain admin-only.
- Tool preview/detail: `Why trust this?`, `Safe install`, `Compare`.
- Toolkit: notes/tags/export/compare.
- Route: `/compare?tools=slug-a,slug-b`.

Do not add a new landing page. The first screen remains the usable discovery
experience.

## 12. Security & Privacy

- All comments, bookmarks, toolkit notes/tags, and exports that depend on user
  state require auth.
- Server functions must use current user from session; never trust user id from
  client payload.
- Use sqlx parameters only.
- No raw HTML injection from tool descriptions, notes, tags, or trust facts.
- Public install guide must not expose `SUPABASE_SERVICE_KEY`, `JWT_SECRET`,
  payout addresses, or admin-only fields.
- x402 remains attribution/trust metadata only.

## 13. Accessibility

- Buttons and tabs must be real semantic controls.
- Touch targets at least 44px on mobile.
- Compare tray must be keyboard reachable and dismissible.
- Copy success uses aria-live or existing copy button announcement pattern.
- Warnings must not rely on color alone.
- No emoji in UI text.

## 14. Implementation Order

Recommended order:

1. **Search & Empty-State Improvements**
   - Immediate usability lift.
   - Pure helpers and existing components, plus `install_risk` filter support
     before any risk-based suggestion ships.
2. **Why Trust This?**
   - Consolidates existing trust sections and supports safe install.
3. **Safe Install / Use With Agent**
   - Converts trust into practical usage.
4. **Compare Tools**
   - Helps final selection.
5. **Tool Finder**
   - Best as a polished entry point after filters/search are stable.
6. **Saved Toolkit Improvements**
   - Depends on compare/install/trust pieces for best value.

If building for maximum user impact first, swap 1 and 5:
Tool Finder first, then empty-state/search polish.

## 15. Testing & Verification

For each phase:

- Unit tests for query mapping, intent parsing, compare limits, risk gating.
- Server tests for public visibility filters and auth-scoped toolkit metadata.
- `cargo fmt --check`
- `cargo check --features ssr`
- `PATH="$HOME/.cargo/bin:$PATH" cargo check --features hydrate --target wasm32-unknown-unknown`
- `cargo clippy --features ssr -- -W clippy::all`
- For UI/auth/routing changes: `./scripts/ui-change-gate.sh`

Browser QA should inspect:

- Desktop 1280x900.
- Mobile 375x812.
- Home, `/tools`, tool detail, compare route, `/toolkit`.

## 16. Open Decisions

1. Compare UI route:
   - Decision: dedicated `/compare?tools=...` route plus sticky tray.
2. Tool Finder placement:
   - Recommended: collapsed panel under the home search, not a full-screen modal.
3. Toolkit export formats:
   - Recommended v1: Markdown + JSON first; MCP config bundle after install guide
     risk gating is stable.
4. Search intent parser:
   - Recommended v1: deterministic keyword parser, not LLM-backed search.
5. Notes/tags schema:
   - Decision default: extend `bookmarks` with note/tags/updated_at. Revisit only
     if migration review finds a concrete reason to split metadata.

## 17. Phase Acceptance Definition

A phase is done only when:

- Non-admin public UX works without stale bundle drift.
- Mobile text does not clip or overlap.
- Existing filters, selected preview, sidebar collapse, auth menu, bookmarks,
  comments, and admin quick actions still work.
- UI gate passes, or any gate failure is explicitly documented with exact error.
