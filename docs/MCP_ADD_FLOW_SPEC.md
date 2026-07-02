# Button-First MCP Add Flow - Implementation Spec

Status: draft
Date: 2026-07-02
Scope: public discovery, tool cards, preview/detail install flow, global MCP connect card, toolkit export handoff

Related docs: [DESIGN.md](../DESIGN.md), [UI_UX_DESIGN.md](UI_UX_DESIGN.md), [BUILD_DEPLOY_RULES.md](BUILD_DEPLOY_RULES.md), [USER_FRIENDLY_DISCOVERY_SPEC.md](USER_FRIENDLY_DISCOVERY_SPEC.md), [X402_REFERRAL_SPEC.md](X402_REFERRAL_SPEC.md)

## 1. Summary

OnchainAI already helps users discover, compare, trust-check, and save crypto tools. The missing product moment is a clear, button-first path from:

> "This tool looks useful" -> "Add it to my agent/client safely."

This spec defines a focused `Add MCP` flow inspired by Component Gallery patterns:

- Button / button group: direct action and platform choice.
- Drawer / sheet: contained install task without losing browse context.
- Popover: compact trust explanation where a full panel would be heavy.
- Progress indicator: short orientation for review -> client -> copy -> save.
- Combobox / guided choice: future discovery refinement, not the core install surface.

The recommended implementation is not a new standalone wizard. It is an intent-aware extension of the existing preview panel and mobile bottom sheet. Tool cards get an `Add MCP` / `Use with agent` action that opens the current preview route with install mode focused.

## 2. Product Position

OnchainAI should feel like a trustworthy package directory for agents, not a crypto app trying to execute transactions.

The install experience must be:

- Fast enough for builders who already know the tool.
- Explainable enough for cautious users deciding whether to paste config.
- Consistent with MCP responses so agents and humans receive the same safety guidance.
- Button-first on the website, but still copy-based under the hood. The site does not run shell commands, install packages, connect wallets, or move funds.

## 3. Current Implementation Facts

This spec builds on existing code. Do not duplicate these surfaces:

- `ToolsBrowser` uses URL-backed `selected=<slug>` state and renders `PreviewPanel` on desktop plus `BottomSheet` on mobile.
- Public filter state already includes `install_risk` across URL parsing, `ToolFilters`, sidebar links, search bar intent parsing, and list SQL.
- `ToolDetailContent` already renders `Why trust this?`, install risk, official links, x402/referral notices, and Generic MCP / Claude / Cursor install tabs.
- `install_safety.rs` already classifies install commands, blocks structured config for high/critical risk, and generates Claude/Cursor guidance.
- `src/server/mcp/install_guide.rs` already returns platform-specific install guide data for MCP clients.
- `/compare?tools=...` exists and loads up to 3 public tools with trust facts and official links.
- `/toolkit` already uses bookmarks, supports notes/tags, and exports Markdown/JSON.
- `PromoCards` has a global `Connect via MCP` card, but it is command-copy-only and not yet platform-button-first.

## 4. Goals

1. Add a visible button-first install path from cards, preview, detail, compare, and toolkit.
2. Reuse existing preview/detail surfaces instead of introducing another modal stack.
3. Make platform choice obvious: Claude, Cursor, Generic MCP, CLI/SDK.
4. Make safety unavoidable before copy: risk level, official evidence, x402 notice, and critical/high-risk behavior.
5. Make the copied content come from one shared install-guide source so UI and MCP do not drift.
6. Preserve existing sidebar, filters, selected preview behavior, auth, bookmarks, compare, comments, and `data-testid`s.

## 5. Non-Goals

- No shell command execution from the browser.
- No automatic package install.
- No wallet connection, custody, x402 facilitator, payment gateway, payment split execution, or transaction submission.
- No public raw `trust_score`.
- No LLM-generated install instructions.
- No new onboarding wall before browsing.
- No replacing `/compare`, `/toolkit`, sidebar filters, chain strip, preview panel, or bottom sheet.

## 6. Benchmark Interpretation

The Component Gallery reference is valuable because it treats components as task primitives, not decorative widgets.

| Pattern | Component Gallery concept | OnchainAI use |
|---|---|---|
| Primary button | Trigger an action | `Add MCP` / `Use with agent` entry point |
| Button group / segmented control | Related actions in one group | Platform selector: Claude / Cursor / Generic MCP / CLI-SDK |
| Drawer / sheet | Contextual task panel | Reuse `PreviewPanel` / `BottomSheet` for add mode |
| Popover | Short floating explanation | Trust/risk reason preview from badges |
| Progress indicator | Discrete task progress | `Review` -> `Client` -> `Copy` -> `Save` orientation |
| Combobox | Search plus option filtering | Later finder/search refinement, not v1 install flow |

References:

- Component Gallery components index: https://component.gallery/components/
- Button: https://component.gallery/components/button/
- Button group: https://component.gallery/components/button-group/
- Drawer: https://component.gallery/components/drawer/
- Popover: https://component.gallery/components/popover/
- Progress indicator: https://component.gallery/components/progress-indicator/
- Combobox: https://component.gallery/components/combobox/

## 7. Design Alternatives Considered

### A. Improve Only The Global Connect Card

Upgrade `Connect via MCP` on the homepage into Claude/Cursor/Generic copy buttons.

Pros:

- Smallest change.
- Helps users add OnchainAI itself as an MCP search provider.

Cons:

- Does not solve adding a specific listed MCP/tool.
- Users still have to open a tool detail page to copy individual config.

Use this as part of v1, not the whole feature.

### B. Polish Only The Existing Safe Install Section

Improve `ToolDetailContent` install tabs and warnings without new card actions.

Pros:

- Very safe and mostly local to detail/preview content.
- Reuses existing architecture.

Cons:

- The install affordance remains buried after card click.
- The user asked for button-style MCP addition, so discovery-to-install is still weak.

Use this as the core panel body, but add entry points.

### C. Recommended: Intent-Aware Add Flow In Existing Preview

Add `Add MCP` / `Use with agent` actions that open the existing preview or bottom sheet with an install-first intent.

Pros:

- Direct button-first behavior.
- Reuses current `selected` route state and overlay surfaces.
- Keeps mobile behavior aligned with existing bottom sheet.
- Avoids nested modal/card complexity.
- Can be built in a narrow branch.

Cons:

- Requires careful URL-state handling to avoid stale SSR/hydration mismatches.
- Requires some refactoring so install guide generation is shared instead of duplicated.

This is the selected direction.

## 8. User Jobs

Primary:

- "I found a specific MCP tool and want to add it to Claude/Cursor."
- "I want to know if copying this config is safe."
- "I want to save this tool for my agent stack."

Secondary:

- "I want to connect OnchainAI itself as an MCP search provider."
- "I want to compare several tools, then install the best one."
- "I want an exportable agent setup checklist from saved tools."

## 9. Information Architecture

### 9.1 Entry Points

Home:

- Keep `Connect via MCP`.
- Change it from a single command row into platform buttons for connecting OnchainAI itself.

Tool list cards:

- Add a compact action in `tool-card-actions`.
- Label rules:
  - `Add MCP` when `tool_type == "mcp"` or `mcp_endpoint` is present.
  - `Use with agent` for CLI/SDK/API tools with an install command.
  - Hide or disable with `No install listed` when no public install path exists.

Preview/detail:

- Show the same install guide panel.
- In normal preview mode, keep the current order: identity, trust, description, install.
- In add mode, reorder or visually prioritize: identity, trust summary, install guide, save/compare/details.

Compare:

- Add an install action per compared tool when an install guide is available.
- Do not make compare a full install wizard.

Toolkit:

- Add `Copy safe MCP config` or `Build config bundle` only after per-tool guide generation is unified and risk-gated.

### 9.2 URL State

Use the existing `selected` parameter and add a small intent parameter:

```text
/tools?type=mcp&selected=zapper-mcp&intent=add-mcp
/?chain=base&selected=bridge-mcp&intent=add-mcp
```

Rules:

- `selected` remains the canonical tool-preview parameter.
- `intent=add-mcp` changes panel focus only; it must not refetch the list.
- Sorting, filters, search, chain, and page query params must be preserved.
- Closing the panel removes `selected` and `intent`.
- Changing filters/sort/search removes `selected` and `intent`.
- SSR/hydration must read router query state, not `window.location`.

## 10. Core Flow

```text
Browse / search / filter
  -> click Add MCP
  -> preview panel or bottom sheet opens in add mode
  -> review trust and risk summary
  -> choose client
  -> copy generated config/command
  -> save to Toolkit or open docs
```

Panel structure in add mode:

1. Tool identity row: logo, name, type, status, install risk.
2. Compact trust strip:
   - Official GitHub / Website / X when verified.
   - Recent activity.
   - Claim status.
   - Evidence gap if links are missing.
3. Install progress:
   - `Review`
   - `Choose client`
   - `Copy`
   - `Save`
4. Platform selector:
   - Claude
   - Cursor
   - Generic MCP
   - CLI/SDK
5. Copy block:
   - Code/config preview.
   - `Copy config` or `Copy command`.
   - Warning or blocked state.
6. Secondary actions:
   - `Save to Toolkit`
   - `Compare`
   - `Open docs`
   - `View full page`

## 11. Components

### 11.1 `AddMcpAction`

Purpose: reusable button/link used by cards, compare rows, detail headers, and toolkit items.

Inputs:

```text
tool: Tool
query_base: String
variant: CardIcon | InlineButton | DetailPrimary
```

Behavior:

- Builds a URL with `selected=<slug>&intent=add-mcp`.
- Preserves the current filter/search query.
- Stops card-click propagation when rendered inside `ToolCard`.
- Uses a real anchor when navigation is URL-backed.
- Uses a real button only when opening an already-mounted panel without route navigation.

Accessibility:

- Icon-only card action needs `aria-label`.
- Visible text variants use `Add MCP` / `Use with agent`.
- Minimum 44px touch target on mobile.

### 11.2 `InstallGuidePanel`

Purpose: shared UI for platform tabs, risk warning, generated config/command, and copy action.

Used by:

- `ToolDetailContent`
- preview add mode
- compare action section
- future toolkit bundle builder

Inputs:

```text
tool: Tool
initial_platform: InstallPlatform
compact: bool
source: Detail | Preview | Compare | Toolkit
```

Requirements:

- Replace duplicated platform/copy markup inside `ToolDetailContent`.
- Use button-group/segmented-control styling, not a tab list that looks like page navigation.
- Keep code/config in `HighlightedCommand` or a code block with stable wrapping.
- Keep `CopyButton` stable: button accessible name does not change after copy; copied feedback remains a live region.

### 11.3 `InstallRiskGate`

Purpose: one risk contract shared by UI and MCP guide responses.

Rules:

- `low`: copy enabled.
- `medium`: copy enabled with caution text.
- `high`: show warning first; user must reveal copy action.
- `critical`: copy blocked and structured config withheld.

Public tool queries already exclude critical tools, but this guard still belongs in the component because the same UI may be reused in admin or edge states.

### 11.4 `TrustEvidenceStrip`

Purpose: compact pre-copy evidence, not a second trust panel.

Shows:

- Install risk label.
- Status: Official / Verified / Community.
- Claim state.
- Last reviewed or recent activity.
- Official links if present.
- Evidence gap if official links are absent.

Do not show raw `trust_score`.

### 11.5 `InstallProgressIndicator`

Purpose: orient the user, not enforce a multi-step wizard.

States:

- `Review`: active until a platform is selected or if risk warning exists.
- `Client`: active while choosing platform.
- `Copy`: active when config/command is visible.
- `Save`: complete if already bookmarked, otherwise secondary.

Implementation may be a compact ordered list styled as a progress tracker. It must not add heavy vertical space to cards or mobile sheets.

### 11.6 `ConnectOnchainAiMcpCard`

Purpose: replace the current global `Connect via MCP` card body.

It connects OnchainAI itself as an MCP search provider, not a third-party tool.

Recommended content:

- Title: `Connect OnchainAI MCP`
- Body: `Let your agent search OnchainAI for crypto tools.`
- Platform buttons:
  - `Claude`
  - `Cursor`
  - `Generic`
- Copy output:
  - Claude/Cursor config JSON where possible.
  - Generic endpoint/command fallback.

Keep this card separate from per-tool `Add MCP`.

## 12. Shared Install Guide Contract

The current UI and MCP guide logic overlap but are not yet a single public contract. V1 should extract or mirror a shared safe guide builder so copied text is consistent.

Proposed public server function:

```text
get_public_install_guide(slug: String, platform: InstallPlatform) -> PublicInstallGuide
```

Proposed model:

```text
PublicInstallGuide {
  slug: String,
  tool_name: String,
  platform: InstallPlatform,
  risk_level: String,
  risk_reasons: Vec<String>,
  warning: Option<String>,
  blocked: bool,
  command: Option<String>,
  config_json: Option<String>,
  copy_text: Option<String>,
  copy_label: String,
  steps: Vec<String>,
  docs_links: Vec<GuideLink>,
  x402_notice: Option<String>,
  referral_disclosure: Option<String>
}

InstallPlatform = Claude | Cursor | GenericMcp | CliSdk
```

Generation rules:

- Prefer `safe_copy_command`.
- If no install command exists but `mcp_endpoint` is an HTTP(S) URL, generate a Generic MCP remote command/config using the known safe `mcp-remote` pattern.
- If a command has high/critical risk, do not generate structured Claude/Cursor config.
- Never invent install commands that are not derivable from stored fields.
- Never expose secrets, service keys, JWT secrets, or admin-only fields.
- x402/referral output is disclosure only. It must not create payment instructions or custody flows.

Open implementation decision:

- Either expose this as a Leptos server function for UI and keep MCP `get_install_guide` delegating to the same builder, or move the builder into a shared server module and call it from both surfaces.
- Recommendation: shared builder first, server function second.

## 13. Platform Behavior

### Claude

Expected output:

- Structured MCP config JSON when safe.
- Steps for where to paste it.
- Fallback command if structured config is unavailable.

Blocked:

- High or critical risk shell wrappers.

### Cursor

Expected output:

- Structured MCP config JSON when safe.
- Steps for opening MCP settings and reloading servers.

Important:

- The current MCP install guide appears to generate Cursor config for the OnchainAI MCP endpoint. For per-tool install, verify whether the target should be the listed tool endpoint/command or the OnchainAI endpoint. The spec requires the label to be explicit:
  - `Connect OnchainAI MCP` for OnchainAI search.
  - `Add this tool to Cursor` for the selected tool.

### Generic MCP

Expected output:

- `mcp_endpoint` if present.
- Safe `npx mcp-remote <endpoint>` command when endpoint is HTTP(S).
- Raw install command only when it is safe/copyable.

### CLI/SDK

Expected output:

- Package install command.
- Link to docs/repo/package.
- No structured MCP config unless an MCP endpoint is present.

## 14. Risk, Trust, And x402 Rules

Trust:

- Public trust is evidence, not a numeric score.
- Show verified official links and human-readable facts.
- If official links are missing, say so plainly.
- Keep `official` as strong proof; `featured` remains editorial and must not imply official status.

Install:

- Critical risk blocks copy.
- High risk requires an explicit reveal.
- Medium risk shows caution.
- Low risk keeps the fast path.

x402:

- x402 is payment/trust metadata only.
- The UI may disclose that calls can request payment.
- The UI must not connect a wallet, facilitate payment, create split fields, or move funds.
- Link to external wallet/agent docs only as informational guidance.

## 15. Visual Design Requirements

Use existing OnchainAI design tokens and style:

- Light theme only.
- Neutral surfaces.
- Orange only for the primary action or focus.
- Cards and buttons keep 8px radius.
- No gradients, crypto-glow styling, or decorative blobs.
- UI copy is English.
- No emoji in UI text.
- Use lucide-style line icons where icon buttons are needed.
- Keep mobile body text readable and touch targets at least 44px.

Desktop:

- Add mode uses the existing 400px preview panel.
- The copy/config block must wrap without horizontal scroll.
- Sticky footer actions are acceptable inside the panel if they do not cover content.

Mobile:

- Add mode uses the existing bottom sheet.
- The bottom sheet should prioritize install guide and trust summary before long descriptions.
- Platform buttons wrap or scroll as a controlled segmented group; they must not shrink text below readable size.

## 16. Accessibility Requirements

- Card action buttons/links must have stable accessible names.
- Platform selector uses real buttons with `aria-pressed` or an equivalent semantic pattern.
- Copy feedback uses the existing live-region pattern.
- Risk warnings use `role="alert"` only when they represent immediate blocking/caution.
- Popovers that contain interactive content use dialog semantics and keyboard dismissal.
- Escape closes preview/add mode.
- Focus should move into the panel when opened and return to the triggering action when closed if feasible.
- Warnings do not rely on color alone.

## 17. Implementation Plan

### Phase 1 - Shared Install Guide Builder

Files likely touched:

- `src/install_safety.rs`
- `src/server/mcp/install_guide.rs`
- `src/server/functions/public_tools.rs`
- new shared model/module if useful

Work:

- Define `InstallPlatform` and `PublicInstallGuide`.
- Extract safe guide generation into a reusable helper.
- Add `mcp_endpoint` fallback generation for Generic MCP.
- Add unit tests for risk gating, platform output, and endpoint fallback.

### Phase 2 - Intent-Aware Preview

Files likely touched:

- `src/components/tools_browser.rs`
- `src/components/preview_panel.rs`
- `src/components/bottom_sheet.rs`
- `src/components/tool_detail_content.rs`

Work:

- Parse `intent=add-mcp`.
- Pass intent into preview/bottom sheet/detail content.
- In add mode, prioritize install guide and compact trust summary.
- Closing the panel removes both `selected` and `intent`.

### Phase 3 - Add MCP Entry Points

Files likely touched:

- `src/components/tool_card.rs`
- `src/pages/compare.rs`
- `src/pages/toolkit.rs`
- possibly new `src/components/add_mcp_action.rs`

Work:

- Add `AddMcpAction`.
- Render it on eligible tool cards.
- Add compare/toolkit install links where space allows.
- Preserve existing bookmark, compare, admin quick actions, card click, and `data-testid`s.

### Phase 4 - Global OnchainAI MCP Connect Card

Files likely touched:

- `src/components/promo_cards.rs`
- possibly new `src/components/connect_mcp_card.rs`

Work:

- Replace command-only row with platform button group.
- Keep the current command/copy fallback.
- Make labels clear that this connects OnchainAI search, not a listed third-party tool.

### Phase 5 - Visual QA And Polish

Work:

- Run local UI gate.
- Inspect desktop 1280x900 and mobile 375x812.
- Check no overlapping text, no horizontal scroll, no stale bundle.

## 18. Acceptance Criteria

Functional:

- A user can click `Add MCP` on an MCP card and land in an install-focused preview/sheet.
- Platform selection updates the generated guide without leaving the panel.
- Copy text matches server-generated guide output.
- High-risk copy requires reveal.
- Critical-risk copy is blocked if such a tool appears in the component context.
- Global `Connect OnchainAI MCP` still works independently of per-tool install.
- Existing card click opens preview as before.
- Existing compare, bookmark/toolkit, filters, search, load more, sidebar state, and auth behavior remain intact.

Security:

- No shell commands execute in browser.
- No secrets or private payment/admin fields are exposed.
- x402 remains disclosure-only.
- Server functions use current session for user-scoped actions.

Accessibility:

- Keyboard users can open, operate, copy, and close the panel.
- Copy success is announced.
- Platform buttons and warnings are understandable without color.

Responsive:

- Desktop panel content fits within 400px without broken layout.
- Mobile bottom sheet keeps 44px targets and readable text.
- Long commands/config wrap or scroll inside code blocks without pushing the page horizontally.

## 19. Verification

Minimum local commands before handoff:

```bash
cargo fmt --check
cargo check --features ssr
PATH="$HOME/.cargo/bin:$PATH" cargo check --features hydrate --target wasm32-unknown-unknown
cargo clippy --features ssr -- -W clippy::all
```

For UI/auth/routing changes:

```bash
./scripts/ui-change-gate.sh
```

Targeted tests to add:

- `PublicInstallGuide` generation:
  - low-risk `npx` -> Claude config copy allowed.
  - high-risk shell wrapper -> structured config blocked.
  - critical command -> copy blocked.
  - HTTP MCP endpoint without install command -> Generic MCP remote command generated.
- URL state:
  - `selected` + `intent=add-mcp` preserved when opening add mode.
  - filters/sort/search remove add mode.
  - close link removes both `selected` and `intent`.
- Component SSR:
  - `AddMcpAction` label differs for MCP vs non-MCP.
  - install guide panel includes risk text before copy action.
  - copy button keeps a stable accessible name.

Browser/visual:

- `/` global connect card.
- `/tools?type=mcp`.
- `/tools?type=mcp&selected=<slug>&intent=add-mcp`.
- `/compare?tools=<a>,<b>`.
- `/toolkit` signed-in and signed-out paths.

## 20. Rollout Strategy

Recommended branch:

```text
codex/mcp-add-flow
```

Suggested order:

1. Merge shared install guide builder and tests.
2. Merge intent-aware preview with no visible card button yet.
3. Add card/detail/compare/toolkit entry points.
4. Polish global `Connect OnchainAI MCP`.
5. Run UI gate and capture screenshots.

If scope needs to shrink, keep only:

- shared guide builder,
- card `Add MCP` action,
- add-mode preview,
- global connect card button group.

Postpone:

- toolkit MCP config bundle,
- popover trust details,
- progress indicator polish,
- combobox search refinement.

## 21. Open Decisions

1. Should the per-tool install guide be loaded through a server function every time, or generated from the already-loaded public `Tool` payload and verified by tests?
   - Recommendation: use a shared server builder and expose it through a server function where fresh guide data matters.
2. Should `intent=add-mcp` change the visual order of `ToolDetailContent`, or only auto-focus/scroll the existing install section?
   - Recommendation: in compact preview/sheet, reorder to install-first; on full detail, preserve the current page order and add a sticky/action jump.
3. Should `mcp_endpoint` fallback generation use `npx mcp-remote` for all HTTP endpoints?
   - Recommendation: yes, but only after URL validation and with tests.
4. Should the progress indicator ship in v1?
   - Recommendation: keep it minimal. Ship if it fits without crowding mobile; otherwise add after the core flow proves useful.
5. Should Toolkit export include an MCP config bundle in v1?
   - Recommendation: no. Add after per-tool guide output is stable and risk-gated.
