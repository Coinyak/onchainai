-- seeds/dev_seed_social.sql — Phase B (auth-dependent) dev seed.
-- profiles + comments + upvotes + bookmarks. See docs/SEED_DATA.md §3.2–3.5, §8.
--
-- PREREQUISITE: profiles.id → auth.users(id) FK is ACTIVE on Supabase. The five
-- UUIDs below must already exist in auth.users. Create them first, e.g. via the
-- Supabase Admin API with the service key, then run this file:
--
--   psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed_social.sql
--
-- On plain Postgres without the `auth` schema the FK is absent and this runs
-- as-is. The fixed UUIDs keep reruns idempotent and assertions stable.

\if :{?seed_env}
\else
  \echo '*** refusing to seed: pass -v seed_env=dev (or test) ***'
  \quit
\endif

SET app.seed_env = :'seed_env';

BEGIN;

DO $$
BEGIN
  IF current_setting('app.seed_env', true) NOT IN ('dev', 'test') THEN
    RAISE EXCEPTION 'refusing to seed: app.seed_env=% (expected dev|test)',
      current_setting('app.seed_env', true);
  END IF;
END $$;

-- ── Profiles ─────────────────────────────────────────────────────────────────
-- Insert the admin FIRST with is_admin=true so the first-user-admin trigger
-- (002_auth) is a no-op for it. onboarding_completed_at set on all but `dave_new`.
INSERT INTO profiles (id, nickname, auth_method, is_admin, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000001', 'satoshi', 'github', true, now())
ON CONFLICT (id) DO NOTHING;

INSERT INTO profiles (id, nickname, auth_method, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000002', 'alice_dev', 'github', now())
ON CONFLICT (id) DO NOTHING;

INSERT INTO profiles (id, nickname, auth_method, wallet_address, chain_id, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000003', 'bob_eth', 'siwx',
        '0x52908400098527886E0F7030069857D2E4169EE7', '1', now())
ON CONFLICT (id) DO NOTHING;

INSERT INTO profiles (id, nickname, auth_method, wallet_address, chain_id, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000004', 'carol_sol', 'siwx',
        '7EYnhQoR9YM3N7UoaKRoA44Uy8JeaZV3qyouov87awMs', 'solana', now())
ON CONFLICT (id) DO NOTHING;

-- newbie: onboarding NULL (exercises the first-login onboarding redirect).
INSERT INTO profiles (id, nickname, auth_method, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000005', 'dave_new', 'email', NULL)
ON CONFLICT (id) DO NOTHING;

-- banned (kept out of all social rows below).
INSERT INTO profiles (id, nickname, auth_method, is_banned, onboarding_completed_at)
VALUES ('00000000-0000-0000-0000-000000000006', 'mallory', 'email', true, now())
ON CONFLICT (id) DO NOTHING;

-- ── Comments (threaded; varied counts per tool) ──────────────────────────────
-- tool_id resolved by slug so this file is independent of tool UUIDs.
-- Counts: uniswap-mcp=3 (1 thread), foundry-dev-cli=5, dune-data-mcp=1, others 0.
INSERT INTO comments (id, tool_id, parent_id, user_id, content)
SELECT v.id, t.id, v.parent_id, v.user_id, v.content
FROM (VALUES
  ('00000000-0000-0000-0000-0000000c0001'::uuid, 'uniswap-mcp',  NULL::uuid,                                   '00000000-0000-0000-0000-000000000002'::uuid, 'Works great with v4 hooks.'),
  ('00000000-0000-0000-0000-0000000c0002'::uuid, 'uniswap-mcp',  '00000000-0000-0000-0000-0000000c0001'::uuid, '00000000-0000-0000-0000-000000000003'::uuid, 'Agreed — routing is fast.'),
  ('00000000-0000-0000-0000-0000000c0003'::uuid, 'uniswap-mcp',  NULL::uuid,                                   '00000000-0000-0000-0000-000000000004'::uuid, 'Would love a Solana adapter.'),
  ('00000000-0000-0000-0000-0000000c0004'::uuid, 'foundry-dev-cli', NULL::uuid,                                '00000000-0000-0000-0000-000000000002'::uuid, 'Indispensable for testing.'),
  ('00000000-0000-0000-0000-0000000c0005'::uuid, 'foundry-dev-cli', NULL::uuid,                                '00000000-0000-0000-0000-000000000003'::uuid, 'forge fuzz is underrated.'),
  ('00000000-0000-0000-0000-0000000c0006'::uuid, 'foundry-dev-cli', '00000000-0000-0000-0000-0000000c0005'::uuid, '00000000-0000-0000-0000-000000000004'::uuid, 'This. Caught two bugs for me.'),
  ('00000000-0000-0000-0000-0000000c0007'::uuid, 'foundry-dev-cli', NULL::uuid,                                '00000000-0000-0000-0000-000000000004'::uuid, 'Docs could be better though.'),
  ('00000000-0000-0000-0000-0000000c0008'::uuid, 'foundry-dev-cli', NULL::uuid,                                '00000000-0000-0000-0000-000000000002'::uuid, 'cast is a Swiss army knife.'),
  ('00000000-0000-0000-0000-0000000c0009'::uuid, 'dune-data-mcp', NULL::uuid,                                  '00000000-0000-0000-0000-000000000003'::uuid, 'Great for dashboards in-agent.')
) AS v(id, slug, parent_id, user_id, content)
JOIN tools t ON t.slug = v.slug
ON CONFLICT (id) DO NOTHING;

-- ── Upvotes (unique per comment+user) ────────────────────────────────────────
INSERT INTO upvotes (comment_id, user_id)
VALUES
  ('00000000-0000-0000-0000-0000000c0001', '00000000-0000-0000-0000-000000000003'),
  ('00000000-0000-0000-0000-0000000c0001', '00000000-0000-0000-0000-000000000004'),
  ('00000000-0000-0000-0000-0000000c0004', '00000000-0000-0000-0000-000000000003'),
  ('00000000-0000-0000-0000-0000000c0005', '00000000-0000-0000-0000-000000000002'),
  ('00000000-0000-0000-0000-0000000c0005', '00000000-0000-0000-0000-000000000004')
ON CONFLICT (comment_id, user_id) DO NOTHING;

-- ── Bookmarks (unique per tool+user) ─────────────────────────────────────────
INSERT INTO bookmarks (tool_id, user_id)
SELECT t.id, v.user_id
FROM (VALUES
  ('uniswap-mcp',     '00000000-0000-0000-0000-000000000002'::uuid),
  ('foundry-dev-cli', '00000000-0000-0000-0000-000000000002'::uuid),
  ('aave-lending-api','00000000-0000-0000-0000-000000000003'::uuid),
  ('lido-staking-mcp','00000000-0000-0000-0000-000000000004'::uuid)
) AS v(slug, user_id)
JOIN tools t ON t.slug = v.slug
ON CONFLICT (tool_id, user_id) DO NOTHING;

COMMIT;

\echo 'Phase B seed complete (profiles, comments, upvotes, bookmarks).'
