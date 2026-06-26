-- scripts/seed-featured-cards.sql — idempotent featured carousel seed.
--
-- Inserts 2–3 active featured_cards rows for approved tools that already exist
-- in `tools`. Preferred slugs are tried first (production catalog names); any slug
-- missing from the DB is skipped automatically.
--
-- Run with psql (service role / direct URL bypasses RLS for INSERT):
--   psql "$DATABASE_URL" -f scripts/seed-featured-cards.sql
--
-- Or via sqlx CLI (same DATABASE_URL in .env):
--   sqlx database execute --file scripts/seed-featured-cards.sql
--
-- Safe to re-run: skips tools that already have a featured_cards row.

BEGIN;

INSERT INTO featured_cards (tool_id, image_url, headline, subtitle, sort_order, is_active)
SELECT
    t.id,
    seed.image_url,
    seed.headline,
    seed.subtitle,
    seed.sort_order,
    true
FROM (
    VALUES
        (
            'quantdinger',
            'https://images.unsplash.com/photo-1611974789855-9c2a0a7236a3?auto=format&fit=crop&w=1200&q=80',
            'Quantitative trading, agent-ready',
            'Featured MCP tooling for onchain markets',
            0
        ),
        (
            'tradingview-mcp',
            'https://images.unsplash.com/photo-1590283603385-17ffb3a7f29f?auto=format&fit=crop&w=1200&q=80',
            'Charts meet MCP',
            'Bridge TradingView signals into your agent stack',
            1
        ),
        (
            'uniswap-mcp',
            'https://images.unsplash.com/photo-1621761190629-da3ca2d3e5e3?auto=format&fit=crop&w=1200&q=80',
            'Swap with Uniswap MCP',
            'Quotes and routing across major EVM chains',
            2
        ),
        (
            'dune-data-mcp',
            'https://images.unsplash.com/photo-1551288049-bebda4e38f71?auto=format&fit=crop&w=1200&q=80',
            'Onchain analytics via Dune',
            'Query dashboards and metrics from your agent',
            3
        ),
        (
            'foundry-dev-cli',
            'https://images.unsplash.com/photo-1555066931-4365d14bab8c?auto=format&fit=crop&w=1200&q=80',
            'Ship contracts with Foundry',
            'Compile, test, and deploy from the terminal',
            4
        )
) AS seed(slug, image_url, headline, subtitle, sort_order)
INNER JOIN tools t
    ON t.slug = seed.slug
   AND t.approval_status = 'approved'
WHERE NOT EXISTS (
    SELECT 1
    FROM featured_cards fc
    WHERE fc.tool_id = t.id
)
ORDER BY seed.sort_order
LIMIT 3;

COMMIT;