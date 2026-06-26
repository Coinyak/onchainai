-- seeds/dev_seed_featured.sql — Phase A featured carousel cards for local dev.
-- Requires Phase A tools from dev_seed.sql (source = 'manual').
-- See docs/SEED_DATA.md and docs/UI_UX_DESIGN.md §12.
--
-- Run:  psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed.sql
--       psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed_featured.sql
-- Reset: psql "$DATABASE_URL" -v seed_env=dev -f seeds/reset.sql

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

-- Idempotent: drop prior dev featured rows tied to manual seed tools.
DELETE FROM featured_cards fc
USING tools t
WHERE fc.tool_id = t.id
  AND t.source = 'manual';

INSERT INTO featured_cards (tool_id, image_url, headline, subtitle, sort_order, is_active)
SELECT t.id,
       '/chains/ethereum.svg',
       t.name,
       'Dev seed — swap tooling on Ethereum',
       0,
       true
FROM tools t
WHERE t.slug = 'uniswap-mcp' AND t.source = 'manual'

UNION ALL

SELECT t.id,
       '/chains/base.svg',
       'Across Bridge',
       'Cross-chain transfers in minutes',
       1,
       true
FROM tools t
WHERE t.slug = 'across-bridge-cli' AND t.source = 'manual'

UNION ALL

SELECT t.id,
       '/chains/arbitrum.svg',
       'Aave Markets',
       'Lending data for major chains',
       2,
       true
FROM tools t
WHERE t.slug = 'aave-lending-api' AND t.source = 'manual';

COMMIT;

\echo 'Featured carousel seed complete (3 active cards when dev_seed.sql tools exist).'