# Chain logo selector + featured carousel — design

Two related homepage features, built together.

- **Feature A — Chain logo selector:** a horizontal strip of official chain logos
  (logos only, no labels) that filters the tool list by chain, replacing the
  sidebar's text "Chain" section. Tool cards render their chains as small logo
  tags.
- **Feature B — Featured carousel:** an admin-managed highlight card below the
  header that rotates through operator-chosen tools (~3s auto-advance, dot
  indicators), each card linking to a single tool's detail page.

Related docs: [UI_UX_DESIGN.md](../../UI_UX_DESIGN.md), [DESIGN.md](../../../DESIGN.md),
[SECURITY.md](../../SECURITY.md), [MVP_DESIGN.md](../../MVP_DESIGN.md).

## Current state (updated 2026-06-27 — implemented / operator hardening branch)

- **Chain strip (A)**: shipped — `src/chains.rs`, `public/chains/*.svg`,
  `ServeDir` at `/chains`, `ChainStrip` in `ToolsBrowser` (home, `/tools`,
  `/categories/*`). Sidebar **no longer** has a Chain `CollapsibleSection`.
- **Tool cards**: catalog chains render as logo `<img>` tags; unknown values
  fall back to text `chain-pill`.
- **Featured carousel (B)**: shipped — `featured_carousel.rs`, migration
  `009_featured_cards.sql`, `/admin/featured`, Supabase `featured` bucket.
  Hidden when zero active cards (operator seed pending).
- **Layout**: site-wide left sidebar with `SidebarBrand` at top (replaces
  `TopNav`). Home **category grid removed**; function discovery via sidebar +
  `/categories/:id`.
- **List depth**: `ToolsBrowser` supports `?page=` progressive Load more
  (50 tools per step, capped at 500 visible) while preserving filters/search.
  Filter and chain changes reset `page` and close preview selection.
- **Mobile visibility**: when no saved preference exists, the sidebar collapses
  by default below tablet width after hydration.
- **SSR hardening**: async/Suspense-rendered navigation controls use normal
  anchors, and the server binary configures a larger Tokio worker stack for
  Leptos SSR chunks.
- **Crawler quality gate**: public visibility now excludes legacy
  migration-backfill-only rows and recovers only rows with strict onchain word
  boundary matches. Generic MCP/API/agent terms are not enough.
- DB chain noise still filtered by `CHAIN_CATALOG` allowlist.

---

## Feature A — Chain logo selector

### A.1 Chain catalog (allowlist)

A single Rust source of truth, `src/chains.rs`:

```rust
pub struct ChainMeta {
    pub id: &'static str,          // canonical id, matches a DB chain value
    pub label: &'static str,       // accessible name (aria-label/title/tooltip)
    pub logo: &'static str,        // "/chains/<file>.svg"
    pub aliases: &'static [&'static str], // other DB values that map here
    pub pinned: bool,              // always shown, even at count 0
}
pub const CHAIN_CATALOG: &[ChainMeta] = &[ /* ordered */ ];
```

- **Allowlist semantics:** only chains in the catalog appear in the strip and as
  card tags. Noise/dead chains are simply absent → auto-excluded, no denylist to
  maintain. A DB chain value matches a catalog entry by `id` or `aliases`.
- **Order:** `bitcoin` (BTC) first (pinned), then `bob` (pinned), then the rest by
  prominence. The strip's primary row shows the first N (≈7 incl. All); the rest
  live behind the `+` tile.
- **Initial catalog** (final inclusion is a content call, trivially editable):
  bitcoin (BTC, pinned), bob (pinned), ethereum, solana, base, arbitrum,
  optimism, polygon, bsc, avalanche, sui, zksync. Excluded as noise/dead:
  `all`, `multi-chain`, `63+ networks`, fantom, litecoin, xrp, celo, gnosis
  (revisit per data).

### A.2 Logo assets (official, downloaded ahead of time)

- Official brand SVGs committed to `public/chains/<id>.svg`, one per catalog entry.
- Sourced from each project's official brand/press kit during implementation and
  checked into the repo (no runtime external fetch, no CDN dependency).
- Served by a new `tower-http` `ServeDir` mounted at `/chains` in `lib.rs`
  (alongside the existing CSS `ServeFile`), with a long cache header.
- Square render: logo centered on a tile with `border-radius`; chain brand mark
  on a white tile. Each `<img>` gets `alt`/`title` = `label`.

### A.3 Strip component (`src/components/chain_strip.rs`)

- Horizontal, horizontally-scrollable row placed at the top of `ToolsBrowser`
  (shared by home tools section, `/tools`, and category pages), above the list.
- First tile: **All** — clears `?chain=`; active (orange border) when no chain
  selected.
- Then catalog tiles (logos only, ~48px squares), BTC first. Multi-select:
  clicking toggles the chain in `?chain=` via the existing `toggle_multi` helper.
  Active tile = 2px `#E76F00` border (matches existing active-filter styling).
- **`+` tile:** inline-expands the strip to reveal the remaining catalog chains
  (no overlay/popover). Collapsed by default; state is client-side.
- Counts come from `get_chain_counts` (already present); pinned chains render even
  at count 0. Counts are not shown as text (logos only) but can drive ordering.
- Accessibility: each tile is a link/button with `aria-label`/`title` = chain
  name and `aria-pressed` for active state (logos carry no visible text).

### A.4 Sidebar change (`src/components/sidebar.rs`)

- Remove the "Chain" `CollapsibleSection` and its rail icon; chain filtering now
  lives entirely in the strip. `function`, `asset_class`, `actor`, `type`,
  `status` sections stay. `?chain=` remains the shared filter param, so existing
  filter wiring (`build_tool_filters`, `ToolFilters`) is unchanged.

### A.5 Tool card tags (`src/components/tool_card.rs`)

- Replace the text `chain-pill` list with small (~20px) chain **logo tags** using
  `CHAIN_CATALOG`. Each tag `<img>` has `title`/`alt` = chain name.
- Chains not in the catalog fall back to the existing small text pill (so nothing
  disappears). Keep an overflow `+N` for long chain lists.

---

## Feature B — Featured carousel

### B.1 Data (`migrations/009_featured_cards.sql`)

```sql
CREATE TABLE featured_cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    image_url TEXT NOT NULL,            -- Supabase Storage public URL
    headline TEXT,                      -- optional overlay title (falls back to tool name)
    subtitle TEXT,                      -- optional
    sort_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```
- RLS: public `SELECT` where `is_active` (active cards are public); INSERT/UPDATE/
  DELETE restricted to `is_admin` profiles (mirrors `site_settings` admin policy
  in `002_auth`).

### B.2 Image storage (Supabase Storage)

- A `featured` Storage bucket. Admin uploads an image through the app; the server
  (service-role key, never exposed to client) stores it and persists the returned
  public URL in `featured_cards.image_url`.
- Upload route is admin-gated server-side (`require_admin`), validates content
  type and size, and returns the stored URL. Image bytes never touch the DB.

### B.3 Admin UI (`/admin/featured`)

- New admin page (pattern follows existing `src/pages/admin/*`): list cards with
  thumbnail, linked tool, active toggle, order; create/edit form with image
  upload, **tool picker** (search by name/slug → `tool_id`), optional
  headline/subtitle, `sort_order`, `is_active`; delete.
- Server fns (admin-guarded via `require_admin`): list/create/update/delete
  featured cards; the upload handler in B.2.

### B.4 Front-end carousel (`src/components/featured_carousel.rs`)

- Large card placed **below the header, above the chain strip** on the home page.
  One card visible at a time: full-bleed `image_url`, optional `headline`/
  `subtitle` overlay; the whole card links to `/tools/<tool.slug>`.
- **Auto-advance every ~3s**, wrapping; **dot indicators** at the bottom
  (clickable to jump; active dot highlighted). Pause on hover/focus (nice-to-have).
- Rotation/dots are client-side (hydrate/wasm); SSR renders the first active card
  and all dots so it is meaningful and crawlable without JS. Uses
  `gloo-timers`/`leptos` signals already in the project.
- Data via a new public server fn `get_featured_cards()` (active, ordered, joined
  to tool slug/name). Hidden entirely when there are no active cards.

### B.5 Public relevance quality gate

- Public catalog queries and RLS hide approved rows whose only evidence is
  `migration-backfill: crypto keyword in name or description` with score 0.
- Recovery migrations use strict word boundaries so generic substrings do not
  pass: `indexes` is not DEX, `define` is not DeFi, and `cryptographic` is not
  crypto.
- Strong recovered examples include explicit Web3/blockchain/chain/protocol
  terms such as Solana, Uniswap, Base USDC, Lightning, CoinGecko, x402, RWA,
  EVM, mainnet/testnet, staking, NFT, and DEX as real terms.

---

## Page layout (home) — as deployed

```
ToolsBrowser (site-layout)
├── Sidebar: SidebarBrand + filter sections (no Chain section)
└── Main:
    ├── (home only) hero: slogan, SearchBar
    ├── (home only) FeaturedCarousel — if active cards exist
    ├── (home only) PromoCards
    ├── ChainStrip
    └── toolbar + tool list
```

- `/tools`, `/categories/:id`: same sidebar + main, **no** hero/carousel/promo.
- **Removed from earlier draft**: `TopNav`, category grid ("Browse by function").
- See `docs/UI_UX_DESIGN.md` §2, §12 for full layout and gaps.

## Data flow

- Filtering is unchanged end to end: strip toggles `?chain=` → `build_tool_filters`
  → `ToolFilters` → `list_tools`/`count_tools`. The strip and tool-card tags are
  pure presentation over `CHAIN_CATALOG` + existing `get_chain_counts`.
- Carousel and chain counts are independent queries; on the home SSR they join the
  already-parallelized load (see the recent per-page query parallelization) rather
  than adding sequential round-trips.

## Error handling

- Missing logo file → broken `<img>`; guard by only referencing files that exist
  (catalog is the source of truth; a build check asserts each `logo` path exists
  under `public/chains/`).
- Carousel: empty/again on fetch error → render nothing (home degrades to no
  carousel). Upload failures return a clear admin error; oversized/invalid types
  rejected server-side.
- Unknown DB chain values (not in catalog): excluded from the strip; on cards they
  fall back to a text pill.

## Testing

- `CHAIN_CATALOG`: unit test that every entry's `logo` file exists, ids/aliases are
  unique, and BTC is first + pinned, BOB pinned.
- Chain mapping: DB value → catalog entry (incl. aliases); noise values map to
  none.
- Featured: server-fn tests for active/ordered selection and admin-only mutation
  (RLS + `require_admin`); upload handler validation (type/size, admin gate).
- Carousel SSR renders first card + dots without JS.
- List pagination helpers cover first-page omission, Load more href generation,
  filter-navigation page reset, and the 500-row visible cap.
- Relevance scanner covers Base network wallet-agent phrasing so real Base
  tools do not stall in manual review solely because the chain was described in
  text rather than source metadata.
- Relevance scanner covers false positives from legacy substring matching
  (`indexes`, `define`, `cryptographic`) so generic developer tools do not
  enter the public crypto catalog.

## Out of scope (YAGNI)

- Per-card scheduling/expiry, A/B weighting, analytics, video cards, multi-tool
  cards, drag-reorder (use a numeric `sort_order` field for now).
