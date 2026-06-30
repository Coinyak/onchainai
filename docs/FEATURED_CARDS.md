# Featured / Highlight Carousel Cards — Operator Playbook

How to promote tools onto the home **highlight carousel** ("프로모 카드") on operator
command, and take the existing ones down. Vendor-neutral: any agent (Claude, Codex,
Cursor) or a human can follow this. Read alongside [OPERATOR_GUIDE.md](OPERATOR_GUIDE.md)
and [SECURITY.md](SECURITY.md).

## What these cards are

- The carousel rendered by `src/components/featured_carousel.rs` on the home page,
  below the hero. Admin label: "Featured Carousel" (`/admin/featured`).
- **Data-backed, not code.** Each card is a row in the `featured_cards` table that
  links to a `tools` row by `tool_id` (image, headline, subtitle, sort_order, is_active).
- The public carousel (`get_featured_cards`) shows only cards where
  `fc.is_active = true` **AND** the linked tool's `approval_status = 'approved'`,
  ordered by `fc.sort_order ASC, fc.created_at ASC`.

## Trigger: "promote X (and Y); take down the existing ones"

Run these steps in **one transaction**. This is **live/production data** (the running
app reads the same DB), so: take down by **deactivating, not deleting** (reversible);
verify after.

### 1. Resolve the tools
Each card needs an **approved** tool. Confirm the slugs exist and are approved:
```sql
SELECT slug, name, approval_status FROM tools
WHERE slug IN ('<slug-a>', '<slug-b>');
```
If a tool is missing or not `approved`, it must be added/approved first (operator
review, or insert as an approved `manual` tool) — **do not silently invent tools.**

### 2. Take down the current cards (reversible)
```sql
UPDATE featured_cards SET is_active = false, updated_at = now()
WHERE is_active = true;
```
Hard `DELETE` only on explicit request — deactivation keeps rows so revert is trivial.

### 3. Put up the new cards
Look the tool up by slug so you never hand-paste a `tool_id`:
```sql
INSERT INTO featured_cards (tool_id, image_url, headline, subtitle, sort_order, is_active)
SELECT t.id, '<IMAGE_URL>', '<Headline>', '<Subtitle>', 0, true
FROM tools t WHERE t.slug = '<slug-a>' AND t.approval_status = 'approved';
-- repeat with sort_order = 1 for the next card, etc. sort_order sets carousel order.
```
`headline` falls back to the tool name when empty. There is **no unique constraint on
`tool_id`** — check for an existing row for that tool first (or `UPDATE` it) to avoid dupes.

### 4. Verify
```sql
-- mirrors get_featured_cards: expect exactly the intended cards, in order
SELECT fc.sort_order, t.slug, fc.headline, fc.image_url
FROM featured_cards fc JOIN tools t ON t.id = fc.tool_id
WHERE fc.is_active = true AND t.approval_status = 'approved'
ORDER BY fc.sort_order, fc.created_at;
```
Then confirm the live render: `curl -s http://localhost:3000/ | grep -o '<img src="[^"]*" alt="[^"]*"'`
and check the headlines/`<img src>` in the carousel.

## Images — use real, product-specific art

The carousel frame is **16:9** with `object-fit: contain` and a transparent
background (`style/output.css` → `.featured-carousel*`), so images are **never
cropped**; non‑16:9 art letterboxes cleanly. Prefer landscape **~16:9** (e.g.
1920×1080 / 1200×630).

**Sourcing priority** (most product-specific first):
1. The tool's **launch/announcement graphic on X** (projects usually post these).
2. Official site **OG image** or blog hero (`curl -s <url> | grep og:image`).
3. **GitHub repo social card**: `https://opengraph.githubassets.com/1/<owner>/<repo>`
   (the `1/` segment is just a cache-buster; returns the repo's card).
4. A bespoke image **uploaded via `/admin/featured`** (stored in the Supabase
   `featured` bucket) when you want self-hosted permanence.

**Pulling an image from an X post without an X API/MCP** — use the public embed
("syndication") endpoint:
```
https://cdn.syndication.twimg.com/tweet-result?id=<TWEET_ID>&token=<TOKEN>&lang=en
```
`TOKEN` is derived from the id:
```js
const token = ((Number(id) / 1e15) * Math.PI).toString(36).replace(/(0+|\.)/g, '');
```
- Photo tweet → `mediaDetails[].media_url_https` (append `?format=jpg&name=large` for full res).
- **Video** tweet (many launch posts) → `mediaDetails[0].media_url_https` is the **poster
  frame** (the title card) — use that.
- A quoted tweet's media may come back empty; fetch the **quoted tweet by its own id**.

**Always** verify before saving:
```
curl -s -o /dev/null -w '%{http_code} %{content_type}\n' '<IMAGE_URL>'   # want: 200 image/*
```
Reliability: `pbs.twimg.com` hotlinks work but depend on X; for permanence, upload via
`/admin/featured` and use that URL.

## Applying SQL

- `DATABASE_URL` is in the repo-root `.env`. It points at a **remote Supabase**
  (treat as production). Use a transaction; deactivate (don't delete); verify after.
- `psql "$DATABASE_URL" -f changes.sql` is the simplest path. If `psql` is absent, a
  one-off Node script with the `pg` package works (`ssl: { rejectUnauthorized: false }`).
- **No-SQL path:** the admin UI `/admin/featured` does all of the above (add / edit /
  delete + image upload) with server-side admin checks — hand this to non-engineers.

## Card layout

Promo art should be ~16:9. The carousel is intentionally capped (`max-width: 720px`,
centered) so a 16:9 banner is not over-tall. If you change images to a different ratio,
they will letterbox (transparent), not crop. Layout lives in `style/output.css`
(`.featured-carousel`, `.featured-carousel-image`); it is served live, no rebuild needed.

## Safety checklist

- [ ] Confirm the exact take-down set and the new set with the operator before writing.
- [ ] One transaction; deactivate (not delete) the old cards.
- [ ] Every `image_url` returns `200 image/*`.
- [ ] New tools are `approved` (else they won't render).
- [ ] Re-run the verify query + live `curl`; report what you saw.
