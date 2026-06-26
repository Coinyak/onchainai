-- seeds/dev_seed.sql — Phase A (auth-free) dev seed.
-- Sources + tools + an optional site_settings demo tweak. No auth dependency:
-- tools.submitted_by stays NULL, so this runs as pure SQL on Supabase today.
-- See docs/SEED_DATA.md.
--
-- Run:  psql "$DATABASE_URL" -v seed_env=dev -f seeds/dev_seed.sql
-- Reset: psql "$DATABASE_URL" -v seed_env=dev -f seeds/reset.sql

\if :{?seed_env}
\else
  \echo '*** refusing to seed: pass -v seed_env=dev (or test) ***'
  \quit
\endif

SET app.seed_env = :'seed_env';

BEGIN;

-- Production guard (hard stop). docs/SECURITY.md prohibits seeding prod.
DO $$
BEGIN
  IF current_setting('app.seed_env', true) NOT IN ('dev', 'test') THEN
    RAISE EXCEPTION 'refusing to seed: app.seed_env=% (expected dev|test)',
      current_setting('app.seed_env', true);
  END IF;
END $$;

-- ── Crawler sources (for /admin/crawler) ──────────────────────────────────────
INSERT INTO sources (name, url, crawl_status, items_found, last_crawled_at, error_message)
VALUES
  ('cryptoskill',   'https://cryptoskill.example/registry', 'success', 42, now() - interval '2 hours', NULL),
  ('github-topics', 'https://github.com/topics/crypto-mcp',  'success', 17, now() - interval '6 hours', NULL),
  ('web3-mcp-hub',  'https://web3mcp.example/hub',           'pending',  0, NULL,                       NULL),
  ('npm',           'https://registry.npmjs.org',            'error',    0, now() - interval '1 day',   'rate limited (403)')
ON CONFLICT (name) DO NOTHING;

-- ── Tools (~19): every function/type/asset_class/status/pricing/actor covered ──
-- `governance` is intentionally left with zero tools (empty-state coverage).
-- created_at is spread so the "New" sort is meaningful; stars spread for "HOT".
INSERT INTO tools
  (name, slug, description, function, asset_class, actor, type,
   repo_url, homepage, install_command, mcp_endpoint, chains, status,
   official_team, trust_score, approval_status, rejection_reason,
   license, pricing, stars, source, created_at)
VALUES
  ('Uniswap MCP', 'uniswap-mcp', 'Swap quotes and routing over Uniswap v4 via MCP.',
   'swap', 'crypto', 'human', 'mcp', 'https://github.com/uniswap/mcp', 'https://uniswap.org',
   'npx @uniswap/mcp', 'npx mcp-remote uniswap.org/mcp', '{ethereum,base,arbitrum}', 'official',
   'Uniswap Labs', 90, 'approved', NULL, 'GPL-3.0', 'free', 4200, 'manual', now() - interval '40 days'),

  ('Across Bridge CLI', 'across-bridge-cli', 'Fast cross-chain transfers across the Across protocol.',
   'bridge', 'crypto', 'human', 'cli', 'https://github.com/across/cli', 'https://across.to',
   'npm i -g @across/cli', NULL, '{ethereum,base,arbitrum,optimism}', 'verified',
   NULL, 70, 'approved', NULL, 'MIT', 'free', 3100, 'manual', now() - interval '35 days'),

  ('Safe Wallet SDK', 'safe-wallet-sdk', 'Programmatic Safe smart-account creation and signing.',
   'wallet', 'crypto', 'human', 'sdk', 'https://github.com/safe-global/sdk', 'https://safe.global',
   'npm i @safe-global/sdk', NULL, '{ethereum}', 'official',
   'Safe', 88, 'approved', NULL, 'LGPL-3.0', 'freemium', 2800, 'manual', now() - interval '30 days'),

  ('x402 Pay', 'x402-pay', 'Pay-per-call settlement for agents over the x402 protocol.',
   'payments', 'stablecoins', 'human', 'x402', 'https://github.com/x402/pay', 'https://x402.org',
   'npx @x402/pay', NULL, '{base}', 'verified',
   NULL, 65, 'approved', NULL, 'Apache-2.0', 'x402', 1500, 'manual', now() - interval '12 days'),

  ('Aave Lending API', 'aave-lending-api', 'Supply, borrow, and liquidation data for Aave v3 markets.',
   'lending', 'crypto', 'human', 'api', 'https://github.com/aave/api', 'https://aave.com',
   NULL, NULL, '{ethereum,arbitrum,polygon}', 'official',
   'Aave', 85, 'approved', NULL, 'MIT', 'free', 2600, 'manual', now() - interval '28 days'),

  ('Lido Staking MCP', 'lido-staking-mcp', 'Stake ETH and query stETH yields via MCP.',
   'staking', 'crypto', 'human', 'mcp', 'https://github.com/lidofinance/mcp', 'https://lido.fi',
   'npx @lido/mcp', 'npx mcp-remote lido.fi/mcp', '{ethereum}', 'verified',
   NULL, 72, 'approved', NULL, 'GPL-3.0', 'free', 1900, 'manual', now() - interval '20 days'),

  ('Hyperliquid Perps SDK', 'hyperliquid-perps-sdk', 'Perpetual futures order management on Hyperliquid.',
   'trading', 'derivatives', 'human', 'sdk', 'https://github.com/hyperliquid/sdk', 'https://hyperliquid.xyz',
   'npm i @hyperliquid/sdk', NULL, '{arbitrum}', 'community',
   NULL, 40, 'approved', NULL, 'MIT', 'paid', 1200, 'manual', now() - interval '9 days'),

  ('OpenSea NFT API', 'opensea-nft-api', 'NFT metadata, listings, and minting across chains.',
   'nft', 'crypto', 'human', 'api', 'https://github.com/opensea/api', 'https://opensea.io',
   NULL, NULL, '{ethereum,base,solana}', 'official',
   'OpenSea', 80, 'approved', NULL, 'MIT', 'freemium', 2200, 'manual', now() - interval '25 days'),

  ('Dune Data MCP', 'dune-data-mcp', 'Query onchain analytics and dashboards from Dune via MCP.',
   'data', 'crypto', 'human', 'mcp', 'https://github.com/duneanalytics/mcp', 'https://dune.com',
   'npx @dune/mcp', 'npx mcp-remote dune.com/mcp', '{ethereum,solana}', 'verified',
   NULL, 68, 'approved', NULL, 'Apache-2.0', 'freemium', 1700, 'manual', now() - interval '15 days'),

  ('Foundry Dev CLI', 'foundry-dev-cli', 'Compile, test, and deploy contracts with Foundry.',
   'dev-tool', 'crypto', 'human', 'cli', 'https://github.com/foundry-rs/foundry', 'https://getfoundry.sh',
   'curl -L https://foundry.paradigm.xyz | bash', NULL, '{ethereum}', 'official',
   'Foundry', 92, 'approved', NULL, 'MIT', 'free', 4000, 'manual', now() - interval '50 days'),

  ('ENS Identity SDK', 'ens-identity-sdk', 'Resolve and manage ENS names and onchain identity.',
   'identity', 'crypto', 'human', 'sdk', 'https://github.com/ensdomains/sdk', 'https://ens.domains',
   'npm i @ensdomains/sdk', NULL, '{ethereum}', 'verified',
   NULL, 64, 'approved', NULL, 'MIT', 'free', 900, 'manual', now() - interval '7 days'),

  ('Farcaster Social Skill', 'farcaster-social-skill', 'Post and read Farcaster casts as an agent skill.',
   'social', 'crypto', 'ai-agent', 'skill', 'https://github.com/farcasterxyz/skill', 'https://farcaster.xyz',
   'npx @farcaster/skill', NULL, '{base}', 'community',
   NULL, 35, 'approved', NULL, 'MIT', 'free', 600, 'manual', now() - interval '5 days'),

  ('Autonomous DeFAI Agent', 'autonomous-defai-agent', 'Self-directed DeFi strategy agent over MCP tools.',
   'ai-agent', 'crypto', 'ai-agent', 'mcp', 'https://github.com/defai/agent', 'https://defai.example',
   'npx @defai/agent', 'npx mcp-remote defai.example/mcp', '{ethereum,base}', 'community',
   NULL, 30, 'approved', NULL, 'MIT', 'free', 800, 'manual', now() - interval '3 days'),

  ('Ondo RWA Markets API', 'ondo-rwa-api', 'Tokenized treasuries and RWA market data.',
   'lending', 'rwa', 'human', 'api', 'https://github.com/ondoprotocol/api', 'https://ondo.finance',
   NULL, NULL, '{ethereum}', 'verified',
   NULL, 60, 'approved', NULL, 'MIT', 'paid', 1100, 'manual', now() - interval '18 days'),

  ('GMX Derivatives CLI', 'gmx-derivatives-cli', 'Open and manage GMX perp positions from the terminal.',
   'trading', 'derivatives', 'human', 'cli', 'https://github.com/gmx-io/cli', 'https://gmx.io',
   'npm i -g @gmx/cli', NULL, '{arbitrum,avalanche}', 'community',
   NULL, 38, 'approved', NULL, 'MIT', 'x402', 700, 'manual', now() - interval '11 days'),

  -- Null/edge row: no description, no install command, empty chains (fallback rendering).
  ('Minimal RPC Tool', 'minimal-rpc-tool', NULL,
   'dev-tool', 'crypto', 'human', 'api', NULL, NULL,
   NULL, NULL, '{}', 'community',
   NULL, 0, 'approved', NULL, NULL, 'free', 0, 'manual', now() - interval '1 day'),

  -- Pending (admin review queue).
  ('Some New Indexer', 'some-new-indexer', 'A community-submitted indexer awaiting review.',
   'data', 'crypto', 'human', 'api', 'https://github.com/example/indexer', NULL,
   NULL, NULL, '{ethereum}', 'community',
   NULL, 0, 'pending', NULL, 'MIT', 'freemium', 0, 'manual', now() - interval '2 days'),

  ('Agent Wallet x402', 'agent-wallet-x402', 'Agent-controlled smart wallet with x402 metering.',
   'wallet', 'crypto', 'ai-agent', 'x402', 'https://github.com/example/agent-wallet', NULL,
   NULL, NULL, '{base}', 'community',
   NULL, 0, 'pending', NULL, 'Apache-2.0', 'x402', 0, 'manual', now() - interval '1 day'),

  -- Rejected (with reason).
  ('Spammy Token Bot', 'spammy-token-bot', 'Auto-shills low-quality tokens.',
   'trading', 'crypto', 'human', 'cli', NULL, NULL,
   NULL, NULL, '{}', 'community',
   NULL, 0, 'rejected', 'Spam / low quality, violates listing policy.', NULL, 'free', 0, 'manual', now() - interval '4 days')
ON CONFLICT (slug) DO NOTHING;

-- Optional admin-settings demo tweak (restore in reset.sql). Comment out to skip.
-- UPDATE site_settings SET slogan = 'Crypto tools, unified. (dev seed)' WHERE id = 1;

COMMIT;

\echo 'Phase A seed complete. Run dev_seed_social.sql after creating auth.users for comments/upvotes/bookmarks.'
