-- 001_init.sql — OnchainAI foundation schema.
--
-- Creates the tools, sources, site_settings, and categories tables with
-- indexes and seed data. Follows MVP_DESIGN.md section 2 schema exactly.
--
-- Targets Supabase Postgres (Postgres 15+). The `extensions` schema is
-- created on Supabase by default; we create it idempotently so the
-- migration also applies to a plain Postgres instance.

-- ---------------------------------------------------------------------------
-- Extensions
-- ---------------------------------------------------------------------------
CREATE SCHEMA IF NOT EXISTS extensions;
CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA extensions;

-- ---------------------------------------------------------------------------
-- Supabase compatibility shim (no-op on real Supabase)
-- ---------------------------------------------------------------------------
-- On Supabase the `anon`/`authenticated` roles, the `auth` schema, and the
-- `auth.uid()` function already exist. On a plain Postgres (e.g. CI/test DB)
-- they do not, so RLS policies referencing them would fail. This shim creates
-- them idempotently only when absent — on Supabase every statement is a no-op.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        CREATE ROLE anon NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN;
    END IF;
END $$;

CREATE SCHEMA IF NOT EXISTS auth;

-- Default auth.uid() returns NULL (no authenticated user) on plain Postgres.
-- On Supabase this function is provided by the platform; this block is a
-- no-op there because the function already exists.
DO $shim$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.routines
        WHERE routine_schema = 'auth' AND routine_name = 'uid'
    ) THEN
        EXECUTE concat(
            'CREATE OR REPLACE FUNCTION auth.uid() RETURNS UUID ',
            'LANGUAGE sql STABLE AS ', chr(36), chr(36),
            ' SELECT NULL::UUID ', chr(36), chr(36)
        );
    END IF;
END $shim$;

-- ---------------------------------------------------------------------------
-- tools
-- ---------------------------------------------------------------------------
CREATE TABLE tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Identity
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,

    -- Classification (3-axis + type)
    function TEXT NOT NULL,                  -- bridge|swap|wallet|payments|lending|staking|trading|nft|data|dev-tool|identity|governance|social|ai-agent
    asset_class TEXT NOT NULL DEFAULT 'crypto', -- crypto|rwa|derivatives|stablecoins
    actor TEXT NOT NULL DEFAULT 'human',     -- human|ai-agent
    type TEXT NOT NULL,                      -- mcp|cli|sdk|api|skill|x402

    -- Connections
    repo_url TEXT,
    homepage TEXT,
    npm_package TEXT,
    install_command TEXT,
    mcp_endpoint TEXT,

    -- Chain support
    chains TEXT[] NOT NULL DEFAULT '{}',

    -- Trust
    status TEXT NOT NULL DEFAULT 'community', -- verified|official|community
    official_team TEXT,
    trust_score INT NOT NULL DEFAULT 0,

    -- Approval (admin panel)
    approval_status TEXT NOT NULL DEFAULT 'approved', -- pending|approved|rejected
    submitted_by UUID,                       -- FK to profiles.id (added in 002_auth)
    rejection_reason TEXT,

    -- Meta
    license TEXT,
    pricing TEXT NOT NULL DEFAULT 'free',    -- free|x402|paid|freemium
    x402_price TEXT,
    stars INT NOT NULL DEFAULT 0,
    last_commit_at TIMESTAMPTZ,

    -- Source
    source TEXT NOT NULL,                    -- cryptoskill|web3-mcp-hub|github|npm|self|manual
    source_url TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes (9 total per MVP_DESIGN.md)
CREATE INDEX idx_tools_function  ON tools(function);
CREATE INDEX idx_tools_asset_class ON tools(asset_class);
CREATE INDEX idx_tools_actor     ON tools(actor);
CREATE INDEX idx_tools_type      ON tools(type);
CREATE INDEX idx_tools_status    ON tools(status);
CREATE INDEX idx_tools_approval  ON tools(approval_status);
CREATE INDEX idx_tools_slug      ON tools(slug);
CREATE INDEX idx_tools_chains    ON tools USING GIN(chains);
CREATE INDEX idx_tools_search    ON tools USING GIN(
    to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
);

-- Auto-update updated_at on row change.
CREATE OR REPLACE FUNCTION tools_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_tools_set_updated_at
    BEFORE UPDATE ON tools
    FOR EACH ROW
    EXECUTE FUNCTION tools_set_updated_at();

-- Row Level Security (public read; write handled after auth tables exist).
ALTER TABLE tools ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Public read tools" ON tools
    FOR SELECT TO anon, authenticated USING (true);

-- ---------------------------------------------------------------------------
-- sources (crawler status tracking)
-- ---------------------------------------------------------------------------
CREATE TABLE sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,               -- cryptoskill|github-topics|...
    url TEXT NOT NULL,
    last_crawled_at TIMESTAMPTZ,
    crawl_status TEXT NOT NULL DEFAULT 'pending', -- pending|success|error
    items_found INT NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sources_name   ON sources(name);
CREATE INDEX idx_sources_status ON sources(crawl_status);

CREATE OR REPLACE FUNCTION sources_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_sources_set_updated_at
    BEFORE UPDATE ON sources
    FOR EACH ROW
    EXECUTE FUNCTION sources_set_updated_at();

ALTER TABLE sources ENABLE ROW LEVEL SECURITY;
-- Public can read source status (crawler dashboard info); writes are server-side only.
CREATE POLICY "Public read sources" ON sources
    FOR SELECT TO anon, authenticated USING (true);

-- ---------------------------------------------------------------------------
-- site_settings (singleton row, admin-managed)
-- ---------------------------------------------------------------------------
CREATE TABLE site_settings (
    id INT PRIMARY KEY DEFAULT 1,
    site_name TEXT NOT NULL DEFAULT 'OnchainAI',
    slogan TEXT NOT NULL DEFAULT 'Crypto tools, unified.',
    description TEXT NOT NULL DEFAULT 'Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.',
    mcp_endpoint TEXT NOT NULL DEFAULT 'npx mcp-remote www.onchain-ai.xyz/mcp',
    search_keywords TEXT[] NOT NULL DEFAULT ARRAY['mcp-server', 'crypto-mcp', 'web3-mcp', 'blockchain-mcp'],
    allow_free_registration BOOLEAN NOT NULL DEFAULT true,
    require_tool_approval BOOLEAN NOT NULL DEFAULT true,
    allow_x402_registration BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Force singleton: only id=1 allowed.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'site_settings_singleton_check'
          AND conrelid = 'site_settings'::regclass
    ) THEN
        ALTER TABLE site_settings ADD CONSTRAINT site_settings_singleton_check CHECK (id = 1);
    END IF;
END $$;

CREATE OR REPLACE FUNCTION site_settings_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_site_settings_set_updated_at
    BEFORE UPDATE ON site_settings
    FOR EACH ROW
    EXECUTE FUNCTION site_settings_set_updated_at();

ALTER TABLE site_settings ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Public read settings" ON site_settings
    FOR SELECT TO anon, authenticated USING (true);
-- Admin update policy added in 002_auth.sql once profiles.is_admin exists.

-- Seed singleton row.
INSERT INTO site_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- categories (14 seed function categories — no emojis, Lucide icon names)
-- ---------------------------------------------------------------------------
CREATE TABLE categories (
    id TEXT PRIMARY KEY,                     -- 'bridge'
    label TEXT NOT NULL,                     -- 'Bridge & Cross-chain'
    icon TEXT NOT NULL,                      -- Lucide icon name: 'git-branch'
    description TEXT NOT NULL,
    sort_order INT NOT NULL
);

CREATE INDEX idx_categories_sort_order ON categories(sort_order);

ALTER TABLE categories ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Public read categories" ON categories
    FOR SELECT TO anon, authenticated USING (true);

-- Seed data (14 function categories from MVP_DESIGN.md section 2).
INSERT INTO categories (id, label, icon, description, sort_order) VALUES
    ('bridge',    'Bridge & Cross-chain', 'git-branch',     'Cross-chain transfers, bridging, wrapping', 1),
    ('swap',      'Swap & DEX',           'arrow-left-right','Token swaps, liquidity, routing',          2),
    ('wallet',    'Wallet & Custody',     'credit-card',    'Wallet creation, management, signing, MPC', 3),
    ('payments',  'Payments',             'dollar-sign',    'Payments, x402, transfers, on/offramp',     4),
    ('lending',   'Lending & Borrowing',  'banknote',       'Lending, borrowing, liquidation',           5),
    ('staking',   'Staking & Yield',      'lock',           'Staking, yield, harvesting',                6),
    ('trading',   'Trading & Perps',      'trending-up',    'Trading, perpetuals, options, copy-trade',  7),
    ('nft',       'NFT & Marketplace',    'image',          'NFT viewing, minting, trading',             8),
    ('data',      'Data & Analytics',     'bar-chart',      'Market data, analytics, indexing, oracles', 9),
    ('dev-tool',  'Developer Tools',      'terminal',       'RPC, indexers, contracts, debugging',       10),
    ('identity',  'Identity & KYA',       'fingerprint',    'Onchain identity, attestation, agent auth', 11),
    ('governance','Governance & DAO',     'vote',           'Voting, proposals, treasury',               12),
    ('social',    'Social & Content',     'message-circle', 'Decentralized social, content, creators',   13),
    ('ai-agent',  'AI Agent',             'bot',            'Autonomous agents, agent economy, DeFAI',   14)
ON CONFLICT (id) DO NOTHING;
