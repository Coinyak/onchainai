# Admin Quick Actions + Carousel Controls — Implementation Spec

Status: draft (ready for approval)
Branch: `feat/devtools-ui-improvements`
Author: agent, from operator ("Codex") proposal + codebase verification

## 1. Summary

Add operator-only "dev tool" affordances to the public UI without introducing a
new permission model, plus polish the featured carousel:

1. **Carousel prev/next arrows** — small circular left/right buttons on the
   featured carousel; existing dots, autoplay, and hover/focus pause stay.
2. **Tool-card admin quick actions** — signed-in `is_admin` users see a small
   `Admin` action row on public tool cards: `Review/Edit` (deep link into the
   review workbench) and `Mark verified` (reuse the audited
   `review_tool(mark_verified)` server path).
3. **Carousel admin manage buttons** — signed-in `is_admin` users see
   `Edit cards` / `Add card` above the carousel, deep-linking into
   `/admin/featured` with the relevant card/tool pre-opened.

All three reuse the **existing server-side admin gate** (`require_admin` inside
`review_tool` and the featured-card server functions). The client `is_admin`
flag is a **UX gate only** — never an authorization boundary.

A hard precondition (§3) is fixed first: the deploy build is currently broken.

## 2. Naming — read this before coding

The home "promo cards" the operator described are, in code, the
**`FeaturedCarousel`** component (`src/components/featured_carousel.rs`), fed by
the `featured_cards` table and managed at `/admin/featured`. It has the dots,
autoplay, and pause behavior referenced in the proposal.

There is a **separate** `PromoCards` component
(`src/components/promo_cards.rs`, rendered at `src/pages/home.rs:59`) — static
marketing tiles, unrelated to this work. Features 1 and 3 target
`FeaturedCarousel`, **not** `PromoCards`. Do not edit `PromoCards`.

## 3. Precondition (P0): fix the hydrate/WASM build — deploy is blocked

The operator said the latest commit is blocked by a Rust build failure. Verified:

| Check | Command | Result |
|---|---|---|
| SSR compile | `cargo check --features ssr` | **green** |
| Client/WASM compile | `~/.cargo/bin/cargo check --features hydrate --target wasm32-unknown-unknown` | **19 errors** |

The deploy path (`cargo leptos build --release`, see `scripts/release-build.sh`)
builds **both** targets, so the hydrate failure blocks release. The SSR check
that agents and the pre-commit hook run is green, which is why this reached
`main` (HEAD `53ea429`) unnoticed.

**Root cause:** server-only helpers that reference `sqlx` (and ssr-only
consts/functions) are **not gated** behind `#[cfg(feature = "ssr")]`, so they get
compiled into the client/WASM build where `sqlx` does not exist. Error sites:

- `src/server/functions/comments_bookmarks.rs:303-304`
  (`sqlx`, `APPROVED_TOOL_ID_BY_SLUG_SQL`)
- `src/server/functions/submissions_workbench/reports_claims.rs`
  (private async helpers `claim_tool_by_slug` @220, `insert_claim_request_row`
  @239, `insert_claim_official_links` @262, `insert_claim_official_link` @317,
  `mark_claim_pending` @336; plus `APPROVED_TOOL_BY_SLUG_SQL`,
  `insert_candidate_official_link`)
- `src/server/functions/submissions_workbench/submission_intake.rs:213-214`
  (`sqlx`)

**Fix:** add `#[cfg(feature = "ssr")]` to each private `sqlx`-touching helper and
to any ssr-only `const`/`fn` those helpers reference. The public `#[server]`
functions themselves are fine — the macro already splits client stub vs server
impl; the bug is only in the hand-written private helpers around them. Mirror the
gating pattern used by helpers elsewhere in these same modules.

**Verification for the fix:** the WASM check above must pass, then the full
`./scripts/ui-change-gate.sh` (which runs the release build) must be green.

**Regression guard (recommended, small):** the agent gate only exercises SSR.
Add the hydrate compile to `scripts/agent-harness-check.sh` (or the pre-commit
hook) so this class of feature-gating break is caught without a full release
build:

```
~/.cargo/bin/cargo check --features hydrate --target wasm32-unknown-unknown
```

Note: this must use the rustup cargo (`~/.cargo/bin/cargo`) — the default `cargo`
on this machine lacks wasm `std` and fails with a misleading "can't find crate
for `core`".

## 4. Goals / Non-goals

**Goals**
- Operator can jump from any public tool card into its review/edit context.
- Operator can verify a tool inline via the existing audited path.
- Operator can reach carousel management (edit existing / add new) from the
  public home page, with the target card/tool pre-opened when possible.
- Carousel gains manual prev/next without losing any current behavior.
- Public UX for non-admins is byte-for-byte unchanged.

**Non-goals**
- No new roles/permissions/claims. Reuse `is_admin` + server `require_admin`.
- No full featured-card create/edit form rendered on the public page (keeps the
  public surface clean). Deep-link into `/admin/featured` instead.
- No `mark_official` from the card (only `mark_verified` this round).
- No x402 payment/custody/wallet UI (project invariant).
- No change to `PromoCards`, sidebar, search, filters, bookmark, or upvote logic.

## 5. Shared infrastructure: one admin flag via context

Today only `TopNav` knows admin state — it calls `get_current_user()` itself
(`src/components/top_nav.rs:152`) and reads `SessionUser.is_admin`
(`src/auth/session.rs:80`). `ToolCard` and `FeaturedCarousel` have no admin
signal. `ToolCard` is rendered in many places (dashboard rails, toolkit,
tools browser), so threading an `is_admin` prop through every call site is
invasive and error-prone.

**Decision:** provide admin state once via Leptos context.

- In `App` (`src/app.rs:157`, which renders `TopNav` once above the router),
  create a single blocking `ArcOnceResource<Option<SessionUser>>` from
  `get_current_user()` and `provide_context` a derived `Signal<bool>` for
  `is_admin` (default `false` on SSR/logged-out).
- Consumers (`ToolCard`, `FeaturedCarousel`) read `use_context::<AdminAccess>()`
  and treat "no context" as non-admin. This keeps them renderable in isolation
  (tests, storybook-style usage) with admin UI simply absent.
- Preserve the documented `TopNav` semantics (blocking SSR so auth is in initial
  HTML; no refetch on client-side nav — auth changes only via full-page
  redirects). Prefer refactoring `TopNav` to consume the same context so there is
  a single auth fetch; if that refactor risks the delicate hydration behavior,
  leaving `TopNav` as-is and adding the context in `App` is acceptable for a
  first pass (documented tradeoff: one extra cached fetch).

**Security note (must hold):** the context flag only shows/hides UI. Every
mutating action still calls a `require_admin`-gated server function. A tampered
client cannot verify a tool or edit cards — the server rejects it.

## 6. Feature 1 — carousel prev/next arrows

File: `src/components/featured_carousel.rs`. CSS: `style/output.css` (carousel
block starts at line 416; input Tailwind source if one drives `output.css`).

**Behavior**
- Render two buttons only when `len > 1` (same guard as the dots at
  `featured_carousel.rs:83`).
- Prev: `current.update(|i| *i = (*i + len - 1) % len)`.
  Next: `current.update(|i| *i = (*i + 1) % len)`.
  Mirrors the autoplay wrap at `featured_carousel.rs:29`.
- Each card is an `<a>`; the buttons overlay it. Handlers MUST call
  `ev.stop_propagation()` and `ev.prevent_default()` (as the dots do at
  `:102-105`) so clicking an arrow paginates instead of navigating.
- The carousel already pauses autoplay on `mouseenter`/`focusin` and resumes on
  leave/out (`:41-44`). Arrow focus lands inside the section, so focus keeps it
  paused for free — no new pause logic. Do not remove those handlers.

**Markup / a11y**
- `<button type="button">` with `aria-label="Previous slide"` /
  `"Next slide"` and a lucide-style chevron SVG (reuse `LucideIcon`,
  `src/components/icons.rs`) — no emoji, no text glyphs.
- Place inside `<section class="featured-carousel">`, siblings of the track, so
  absolute positioning is relative to the carousel box.
- Keyboard: buttons are natively focusable and Enter/Space activate. Provide a
  visible `:focus-visible` ring consistent with `.carousel-dot:focus-visible`
  (`output.css:492`).

**Styling** (calm theme, orange only for interaction/focus)
- `.carousel-arrow`: absolute, vertically centered
  (`top:50%; transform:translateY(-50%)`), `left:8px` / `right:8px`, ~32-36px
  circle, `background: rgba(255,255,255,0.85)`, subtle border, `pointer-events:
  auto`, icon in `#1A1A1A`. Hover slightly darker surface; `:focus-visible` uses
  the orange/white ring already used by dots.
- Must not overlap the bottom dots or the overlay headline — vertical center
  keeps them clear of the bottom gradient (`.featured-carousel-overlay`).
- Touch target ≥44px effective (pad the hit area even if the visual circle is
  smaller) per the mobile invariant.

**Do not** change the 16:9 image framing, `object-fit: contain`, or the 720px
max-width — these were deliberate (`output.css:418-449`).

## 7. Feature 2 — tool-card admin quick actions

File: `src/components/tool_card.rs`. Server: `review_tool`
(`src/server/functions/admin_review.rs:543`).

**Visibility**
- Read `is_admin` from context (§5). When false/absent, render nothing new — the
  card is unchanged for everyone else.
- The actions belong in the existing `.tool-card-actions` cluster
  (`tool_card.rs:277`) or a new sibling `.tool-card-admin` row, kept visually
  distinct and secondary (small, neutral). Reuse the card action button pattern.

**Actions**

1. `Review / Edit` → link to `/admin/tools?selected=<slug>`.
   - Caveat (must handle or document): the workbench resolves `selected` only if
     the slug is present in the **currently active queue's** list
     (`derive_selected_slug`, used at `admin/tools.rs:89-97`; default queue is
     `new_candidate`). An already-public tool is usually in **no** review queue,
     so `?selected=<slug>` alone may land on the workbench with nothing selected.
     Options: (a) accept "lands on workbench" for v1 and note it; (b) add a
     direct `?slug=<slug>` load path to `AdminToolsContent` that fetches the
     workbench for any slug regardless of queue membership (small server reuse of
     `get_admin_tool_workbench`). Recommend (b) if we want the link to always
     open the tool; otherwise ship (a) and file a follow-up.

2. `Mark verified` → call `review_tool(ReviewToolPayload { action: "mark_verified", .. })`.
   - `mark_verified` **requires a non-empty reason** (validated at
     `admin_review.rs:414-420`). Supply a fixed default, e.g.
     `"Verified via public-card quick action"`. The server already records the
     acting `admin_id`; no reason prompt is needed for this inline path.
   - Payload shape (from `ReviewToolPayload`, see `admin/tools.rs:437-445`):
     `slug`, `action`, `reason`, and `None` for `override_reason`,
     `expected_updated_at`, `snapshot_id`, `recommendation_id`.
   - `mark_verified` sets listing status to `verified` (status-only transition,
     `operator_review_transition.rs:85`) — it does not touch claim state.
   - On success: reflect the new state on the card (badge → Verified) and
     hide/disable the button. On error: surface a small inline message; do not
     silently swallow. Use a busy flag to prevent double-submit (mirror
     `admin/tools.rs:126-144`).

**Duplicate-action rules**
- If `tool.status == "verified"`: hide/disable `Mark verified`, show current
  status. If `tool.status == "official"`: same (official already outranks
  verified). Never present an action that would be a no-op or a downgrade.

**A11y / design**
- Real `<button>`/`<a>` elements, English labels, no emoji, lucide icons.
- Keep the actions out of the card's main `<a>` (the link wraps
  `tool-card-inner`, `:199`); place them in the sibling actions container so they
  are not nested interactive-in-interactive. Stop propagation on click.

## 8. Feature 3 — carousel admin manage buttons + deep links

Files: `src/components/featured_carousel.rs` (buttons + deep-link),
`src/pages/admin/featured.rs` (accept query params).

**Public side**
- When `is_admin` (context), render an `Edit cards` / `Add card` control row
  **above** the carousel (or as an overlay affordance on the current card). Keep
  it small and clearly operator-tinted; non-admins never see it.
- `Add card` → `/admin/featured?new=1` (optionally `&tool=<current card slug>` to
  prefill the tool for the visible card's tool).
- `Edit cards` / per-card edit → `/admin/featured?edit=<card.id>`. The public
  `FeaturedCardView` already carries `id: Uuid` and `tool_slug`
  (`taxonomy_featured/featured.rs:6-15`), so both deep links are possible with no
  new server data.

**Admin side (`/admin/featured` enhancement)**
- `AdminFeaturedPage` currently ignores query params. Add `use_query_map` and:
  - `?new=1` (or `?tool=<slug>`) → open the create form (`show_create=true`),
    prefilling the tool when `tool` is present (reuse the picker/`on_pick`
    plumbing, `admin/featured.rs:245-249`).
  - `?edit=<uuid>` → set `editing_id` to that card so its inline edit row expands
    (`admin/featured.rs:93-108`), and scroll it into view.
- Parse defensively: invalid/unknown ids are ignored (page renders normally).
  Do not error the page on a stale deep link.

This stays within the "no full form on the public page" non-goal — the public UI
only links; all editing remains inside the gated admin page.

## 9. Design & accessibility invariants (from DESIGN.md / AGENTS.md)

- UI text English; **no emoji**; lucide-style line icons only.
- Calm light theme; **orange (`--accent-text` / `#E76F00`) only** for primary
  interaction/focus. Admin affordances stay neutral/secondary, not loud.
- `#E76F00` fails WCAG AA on white for small text — use the existing
  `--accent-text` (`#B35000`) token for any accent text, per `output.css:25-29`.
- Touch targets ≥44px; mobile body text stays ≥16px.
- Card radius ≤8px (the carousel's 16px is a pre-existing, documented exception —
  do not "fix" it here).
- Preserve every existing `data-testid`, and the bookmark/upvote/login-modal
  behaviors in `ToolCard`.

## 10. Testing & verification

Per `.claude/skills/onchainai-ui-workflow` and the AGENTS verification matrix:

1. **Unit** (`cargo test --features ssr`): pure helpers only — e.g. the
   prev/next index math (extract a small `fn wrap_index(cur, len, dir)` and test
   wrap-around both directions), and any deep-link query builder. Keep DOM out of
   unit tests.
2. **SSR compile**: `cargo check --features ssr`.
3. **Client compile (the one that catches §3):**
   `~/.cargo/bin/cargo check --features hydrate --target wasm32-unknown-unknown`.
4. **Iterate** with `./scripts/dev-watch.sh` (never `cargo build --features ssr`
   for UI preview — stale bundle).
5. **Final gate**: `./scripts/ui-change-gate.sh` (release build + restart +
   bundle-timestamp + curl/browser/auth smoke + desktop/mobile screenshots).
6. **Manual matrix** at `1280x900` and `375x812`, logged-out vs admin:
   - Logged-out: no admin row on cards, no manage buttons, carousel arrows work.
   - Admin: quick actions present; `Mark verified` flips badge and disables;
     deep links open the right workbench/card; arrows + dots + autoplay + pause
     all still work.

State exactly which commands ran and their result in the handoff. Do not claim
browser/visual QA passed unless the gate actually ran.

## 11. Sequencing

0. **P0:** fix hydrate feature-gating (§3) + add the hydrate check to the harness.
   Land/verify this first — nothing deploys until it is green.
1. Shared admin context (§5).
2. Feature 1 (carousel arrows) — self-contained, lowest risk.
3. Feature 2 (tool-card quick actions) — depends on §5; decide the `?selected`
   vs `?slug` caveat (§7).
4. Feature 3 (carousel manage + `/admin/featured` query params).

Each step is a small, reviewable change. Run the client compile after any change
touching `src/server/**` to avoid re-introducing a §3-class break.

## 12. Risks & open questions

- **`?selected=<slug>` may not open arbitrary public tools** in the workbench
  (§7). Decision needed: ship "lands on workbench" (a) or add a `?slug=` direct
  loader (b). Recommend (b).
- **Single vs double auth fetch** if `TopNav` is not refactored onto the shared
  context (§5). Low impact (cached blocking resource); refactor when safe.
- **Inline `Mark verified` has no undo prompt** — it is one click with a fixed
  reason. That matches "quick action", but if operators want a confirm step,
  reuse the existing `ReasonModal` pattern (`admin/tools.rs:400-455`). Confirm
  desired friction level.
- **Arrow placement vs overlay/dots** on very short viewports — verify no
  overlap with the bottom gradient/headline on mobile screenshots.
