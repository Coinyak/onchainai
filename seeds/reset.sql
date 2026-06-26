-- seeds/reset.sql — remove dev/test seed rows. Idempotent.
-- Never touches `categories` (shipped by migration) or crawler-managed state
-- beyond what the seed created. See docs/SEED_DATA.md §10.
--
-- Run: psql "$DATABASE_URL" -v seed_env=dev -f seeds/reset.sql

\if :{?seed_env}
\else
  \echo '*** refusing to reset: pass -v seed_env=dev (or test) ***'
  \quit
\endif

SET app.seed_env = :'seed_env';

BEGIN;

DO $$
BEGIN
  IF current_setting('app.seed_env', true) NOT IN ('dev', 'test') THEN
    RAISE EXCEPTION 'refusing to reset: app.seed_env=% (expected dev|test)',
      current_setting('app.seed_env', true);
  END IF;
END $$;

-- Phase B seed rows. Bookmarks/upvotes/comments cascade from tools/profiles,
-- but delete explicitly so a profiles-only reset still clears them.
DELETE FROM bookmarks b USING tools t
  WHERE b.tool_id = t.id AND t.source = 'manual';
DELETE FROM upvotes u USING comments c, tools t
  WHERE u.comment_id = c.id AND c.tool_id = t.id AND t.source = 'manual';
DELETE FROM comments c USING tools t
  WHERE c.tool_id = t.id AND t.source = 'manual';

-- Phase A seed tools (everything we inserted is source = 'manual').
DELETE FROM tools WHERE source = 'manual';

-- Restore the optional site_settings demo tweak.
UPDATE site_settings SET slogan = 'Crypto tools, unified.' WHERE id = 1;

-- NOTE: crawler `sources` rows are intentionally left in place — the crawler
-- manages them and they are harmless. Remove manually if needed.

COMMIT;

\echo 'Seed reset complete (source=manual tools and their social rows removed).'
