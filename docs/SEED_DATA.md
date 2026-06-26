# Seed & Test Data Spec

Defines the **seed data** for local development and the **fixture data** for
automated tests. Closes the gap surfaced during verification: a fresh dev DB
contains only the crawler self-registered `OnchainAI` tool, so list, category,
comment-count, and admin-queue screens cannot be verified with realistic data.

Read alongside [MVP_DESIGN.md](MVP_DESIGN.md) (schema), [SECURITY.md](SECURITY.md)
(RLS, never seed prod), and the migrations in `/migrations`.

## 1. Goals

- Every browse/detail/admin screen renders with realistic data on a fresh dev DB.
- Exercises the comment-count batch path (`get_tool_comment_counts`) — multiple
  tools, varied non-zero counts, threaded replies.
- Covers every enum value at least once (function, type, asset_class, status,
  pricing, actor, approval_status, auth_method) so filters and badges are testable.
- Deterministic and idempotent: rerun without duplicates or FK breakage.
- **Never runs against production.** Hard env gate (Section 7).

## 2. Principles

| Principle | Rule |
|---|---|
| Schema-aligned | Columns/enums match `/migrations` exactly. No invented fields. |
| Deterministic UUIDs | Fixed UUIDs from a `seed:` namespace (UUIDv5 or hand-assigned `00000000-0000-0000-0000-0000000000NN`) so FKs resolve and reruns are stable. |
| Idempotent | Every insert uses `ON CONFLICT … DO NOTHING` (or `DO UPDATE` for mutable demo fields). |
| Layered | `categories` already ship in `001_init`. Seed adds only profiles → tools → comments → upvotes → bookmarks, in FK order. |
| Env-gated | Refuses to run unless `SEED_ENV in (dev, test)`. |
| Realistic | Names/descriptions/install commands resemble real crypto tooling, not `foo`/`bar`. |

## 3. Data Sets

> **Two-phase seeding (critical).** On Supabase the `profiles.id` → `auth.users(id)`
> FK is **active** (migration `002_auth`, added when the `auth` schema exists), so
> anything referencing a user cannot be inserted with plain SQL alone.
> - **Phase A — auth-free (pure SQL):** `categories` (shipped), `site_settings`
>   (shipped), `sources`, and **`tools`** (`submitted_by` is nullable → set it
>   `NULL`). This phase alone populates home, `/tools`, category pages, and the
>   admin tool queue — i.e. it fixes the "empty DB" symptom.
> - **Phase B — auth-dependent:** `profiles` + `comments` + `upvotes` +
>   `bookmarks`. Requires real `auth.users` rows first (Supabase Admin API with
>   the service key, or a test-only insert into `auth.users`). Only this phase
>   produces non-zero comment counts. See Section 8.
>
> On plain Postgres without the `auth` schema the FK is absent, so both phases run
> as pure SQL.

### 3.1 Categories — already seeded
The 14 `function` categories are inserted by `001_init.sql` (`bridge`, `swap`,
`wallet`, `payments`, `lending`, `staking`, `trading`, `nft`, `data`, `dev-tool`,
`identity`, `governance`, `social`, `ai-agent`). Seed data **references** these
ids; it must not re-insert or alter them.

### 3.2 Profiles (5) — Phase B
`profiles.id` is the auth user id (Section 8). Nicknames must satisfy
`^[a-zA-Z0-9_-]+$`, 2–20 chars. Set `onboarding_completed_at` (column added in
`004_onboarding`) to a non-NULL value on **all but one** seed profile — otherwise
seeded users are bounced into the first-login onboarding flow. Reserve one
profile with `onboarding_completed_at = NULL` to exercise that flow.

| key | nickname | auth_method | role | onboarding | notes |
|---|---|---|---|---|---|
| admin | `satoshi` | github | `is_admin=true` | done | insert **first**; reviews pending tools |
| dev1 | `alice_dev` | github | user | done | comments + bookmarks |
| dev2 | `bob_eth` | siwx (EVM, chain_id `1`) | user | done | upvotes; valid checksummed EVM addr |
| dev3 | `carol_sol` | siwx (Solana) | user | done | replies; valid base58 Solana addr |
| newbie | `dave_new` | email | user | **NULL** | exercises onboarding redirect |

A separate `mallory` profile with `is_banned=true` exercises the ban gate; keep it
out of all comment/upvote/bookmark seed rows.

### 3.3 Tools (~18) — Phase A
Span **all** `function` ids, with each `type`, `asset_class`, `status`,
`pricing`, and `actor` value appearing at least once. `submitted_by = NULL`.

- **15 `approved`** — populate the public list/category pages.
- **2 `pending`** — populate the admin review queue (`/admin/tools`).
- **1 `rejected`** (with `rejection_reason`) — exercises the rejected state.
- `stars` spread 0–4200 so the **HOT** sort is meaningful.
- `created_at` spread across days/weeks so the **New** sort is meaningful (don't
  let all rows share `now()`).
- `chains` arrays vary: `{ethereum}`, `{solana}`, `{ethereum,base,arbitrum}`, `{}`.
- A few set `official_team` + `status='official'`; a few `status='verified'`.
- `source='manual'` for all seed tools (distinguishes from crawler rows; avoids
  colliding with the `source='self'` self-registered tool).
- **Empty-state coverage:** leave exactly one category (e.g. `governance`) with
  **zero** approved tools so its category page and a filtered list render the
  empty state and its sidebar count shows 0.
- **Null/edge rows:** include 1–2 approved tools with `description = NULL`,
  `install_command = NULL`, and `chains = '{}'` to verify fallback rendering
  (the install block hides, the chains row collapses).
- **Boundary text:** one tool with a max-length name and one with the longest
  realistic description; **no emoji** (UI rule, AGENTS.md). Unicode/CJK allowed.
- **Pagination:** list queries `LIMIT 50`. The ~18-row dev set does not paginate;
  use the `perf` profile (Section 5) to exercise pagination and the batch
  comment-count query under volume.

### 3.4 Comments (~25, threaded) — Phase B
- Distribute across **at least 6 different tools** with **varied counts**
  (e.g. 0, 1, 3, 5, 8) so `get_tool_comment_counts` returns a non-uniform map and
  the batch-vs-N+1 behavior is observable.
- Include **top-level + replies** (`parent_id` set) to test thread rendering.
- Authored by dev1/dev2/dev3 (never the banned user).

### 3.5 Upvotes & Bookmarks — Phase B
- Upvotes on ~10 comments, multiple users per comment. Respect
  `upvotes_comment_user_unique (comment_id, user_id)` →
  `ON CONFLICT (comment_id, user_id) DO NOTHING`.
- Bookmarks: each non-banned user bookmarks 2–3 tools. Respect
  `bookmarks_tool_user_unique (tool_id, user_id)` →
  `ON CONFLICT (tool_id, user_id) DO NOTHING`.

### 3.6 site_settings — already seeded (Phase A, optional override)
The singleton row (`id = 1`) ships with defaults via `001_init` and is updated by
`005`. Seed need not insert it. Optionally `UPDATE` `slogan`/`description` or the
admin toggles (`allow_free_registration`, `require_tool_approval`,
`allow_x402_registration`, `search_keywords`) to demo the `/admin/settings` page;
restore defaults in `reset.sql`.

### 3.7 sources (~4) — Phase A, auth-free
Seed the crawler `sources` rows (`cryptoskill`, `github-topics`, `web3-mcp-hub`,
`npm`) with mixed `crawl_status` (`success`/`error`/`pending`) and `items_found`
so `/admin/crawler` renders a realistic dashboard. `name` is UNIQUE →
`ON CONFLICT (name) DO NOTHING`.

## 4. Determinism

Assign stable UUIDs so cross-table FKs are writable in one pass and reruns are
no-ops. Convention: `00000000-0000-0000-0000-0000000000NN` for low-count seed
rows, or `uuid_generate_v5(<seed-namespace>, '<stable-key>')`. Tools may instead
key on their unique `slug` for `ON CONFLICT`.

## 5. Volume Targets

| Profile | Tools | Comments | Use |
|---|---|---|---|
| `dev` | ~18 | ~25 | manual UI verification (this spec's default) |
| `test` | minimal per-case | per-case | deterministic fixtures, see Section 9 |
| `perf` (optional) | 5k generated | 50k generated | N+1 / pagination load testing |

## 6. Layout & Application

```
migrations/            # schema only — unchanged
seeds/
  dev_seed.sql         # Phase A — auth-free: sources, tools, site_settings demo
  dev_seed_social.sql  # Phase B — needs auth.users: profiles, comments, upvotes, bookmarks
  reset.sql            # truncate/delete seed rows (Section 10); never categories
```

Apply (dev):
```bash
SEED_ENV=dev psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed.sql
# Phase B only after auth.users exist (Section 8):
SEED_ENV=dev psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed_social.sql
```

Both files MUST:
1. Abort unless the gate passes (Section 7).
2. Run inside a single `BEGIN … COMMIT` so a partial failure rolls back cleanly.
3. Insert in FK order — A: sources → tools (+ optional `site_settings` update);
   B: profiles → comments → upvotes → bookmarks.
4. Use `ON CONFLICT DO NOTHING` everywhere (mutable demo fields may `DO UPDATE`).

## 7. Production Guard (required)

The seed file refuses to run outside dev/test. Example guard at the top of
`dev_seed.sql`:

```sql
DO $$
BEGIN
  IF current_setting('app.seed_env', true) IS DISTINCT FROM 'dev'
     AND current_setting('app.seed_env', true) IS DISTINCT FROM 'test' THEN
    RAISE EXCEPTION 'refusing to seed: app.seed_env=% (set to dev|test)',
      current_setting('app.seed_env', true);
  END IF;
END $$;
```
Invoke with `-v` / `SET app.seed_env = 'dev';` prepended by the runner. Seeding
prod data is prohibited by [SECURITY.md](SECURITY.md); this is a hard stop, not a
convention.

## 8. Caveats

- **`profiles.id` ↔ `auth.users`** — `002_auth` adds this FK **only when the
  `auth` schema exists**, which it does on Supabase. The project's dev DB is
  Supabase, so the FK is active and Phase B cannot be pure SQL. Pick one path and
  record it before writing the Phase-B seed:
  1. **Supabase Admin API (recommended):** a small script uses the service key to
     create `auth.users` (email/SIWX placeholder users), captures their ids, then
     runs the profile/social SQL bound to those ids.
  2. **Direct `auth.users` insert (test/local only):** insert the minimal required
     `auth.users` columns first in the same transaction. Brittle across GoTrue
     versions; acceptable only for a disposable local DB.
  Phase A (tools/sources/site_settings) has **no** auth dependency and runs as
  pure SQL on Supabase today.
- **No real PII** — seed emails use `@example.com`; SIWX wallet addresses are
  syntactically valid but throwaway (checksummed EVM / base58 Solana). Never seed
  real user data.
- **First-user-admin trigger** — `002_auth` flips the first profile to admin on
  insert. Insert the intended admin (`satoshi`) **first**, or set `is_admin`
  explicitly and assume the trigger is a no-op on conflict.
- **Crawler self-register** — startup inserts the `OnchainAI` tool
  (`source='self'`). Seed tools use `source='manual'` and distinct slugs; rely on
  `ON CONFLICT (slug) DO NOTHING` so a re-run after the crawler is safe.
- **RLS** — seeding runs as the service role / DB owner, bypassing RLS. Never
  hand the seed file a public/anon connection.

## 9. Test Fixtures (`test` profile)

Integration tests do **not** load `dev_seed.sql`. They build the **minimal**
rows each case needs (one tool + two comments to assert a count of 2), inside a
transaction that rolls back. Keep fixtures in the test module, deterministic, and
independent — no shared mutable seed state between tests. Reuse the same fixed
UUIDs so assertions can reference rows by id.

## 10. Reset / Teardown

`seeds/reset.sql` truncates `bookmarks, upvotes, comments` then
`DELETE FROM tools WHERE source='manual'` and `DELETE FROM profiles WHERE id IN (…seed ids…)`.
It must **never** touch `categories` (shipped by migration) or crawler-sourced
tools.

## 11. Example Rows (schema-accurate)

```sql
-- profile (admin)
INSERT INTO profiles (id, nickname, auth_method, is_admin)
VALUES ('00000000-0000-0000-0000-000000000001', 'satoshi', 'github', true)
ON CONFLICT (id) DO NOTHING;

-- approved tool
INSERT INTO tools
  (name, slug, description, function, asset_class, actor, type,
   install_command, chains, status, official_team, trust_score,
   approval_status, license, pricing, stars, source)
VALUES
  ('Uniswap MCP', 'uniswap-mcp',
   'Swap quotes and routing over Uniswap v4 via MCP.',
   'swap', 'crypto', 'human', 'mcp',
   'npx @uniswap/mcp', '{ethereum,base,arbitrum}', 'official', 'Uniswap Labs',
   90, 'approved', 'GPL-3.0', 'free', 4200, 'manual')
ON CONFLICT (slug) DO NOTHING;

-- pending tool (admin queue)
INSERT INTO tools
  (name, slug, description, function, type, approval_status, pricing, source)
VALUES
  ('Some New Indexer', 'some-new-indexer', 'Pending review.',
   'data', 'api', 'pending', 'freemium', 'manual')
ON CONFLICT (slug) DO NOTHING;

-- threaded comments on a tool (drives non-zero comment count)
INSERT INTO comments (id, tool_id, parent_id, user_id, content)
SELECT '00000000-0000-0000-0000-0000000000c1', t.id, NULL,
       '00000000-0000-0000-0000-000000000002', 'Works great with v4.'
FROM tools t WHERE t.slug = 'uniswap-mcp'
ON CONFLICT (id) DO NOTHING;
```

## 12. Acceptance Checklist

- [ ] `/tools` shows ≥15 approved tools across multiple functions and statuses.
- [ ] Category pages each show their tools; counts in the sidebar are non-zero.
- [ ] Tool cards display **varied** comment counts (not all 0); batch query issues
      one round-trip for the page.
- [ ] `/admin/tools` lists the pending + rejected tools.
- [ ] Every filter axis (function, asset_class, actor, type, status) has ≥1 match.
- [ ] Banned user cannot authenticate (gate hit); their rows are absent from seed
      comments.
- [ ] Re-running the seed produces zero new rows (idempotent).
- [ ] Seed aborts when `app.seed_env` is unset or `production`.
