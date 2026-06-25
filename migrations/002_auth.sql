-- 002_auth.sql — OnchainAI auth & profiles schema.
--
-- Creates the `profiles` table (with first-user-admin trigger), the
-- `siwx_sessions` table, the `profiles_public` security_invoker view, and
-- applies the RLS policies from SECURITY.md section 4.2. Also wires the
-- `tools.submitted_by` foreign key declared in 001_init.sql.

-- ---------------------------------------------------------------------------
-- profiles
-- ---------------------------------------------------------------------------
-- `id` mirrors auth.users(id) on Supabase. On plain Postgres without the
-- auth schema, the FK is optional — we add it only if auth.users exists.
CREATE TABLE profiles (
    id UUID PRIMARY KEY,
    nickname TEXT UNIQUE,                    -- 2-20 chars, ^[a-zA-Z0-9_-]+$
    bio TEXT,                                -- max 200 chars (validated app-side)
    avatar_url TEXT,
    auth_method TEXT NOT NULL,               -- github|email|siwx
    wallet_address TEXT,                     -- EVM/Solana address (siwx only)
    chain_id TEXT,                           -- '1' (EVM) | 'solana' (siwx only)
    is_admin BOOLEAN NOT NULL DEFAULT false,
    is_banned BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_profiles_nickname      ON profiles(nickname);
CREATE INDEX idx_profiles_auth_method   ON profiles(auth_method);
CREATE INDEX idx_profiles_wallet_address ON profiles(wallet_address);

-- Link to auth.users when running on Supabase (idempotent).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'auth' AND table_name = 'users'
    ) THEN
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.table_constraints
            WHERE constraint_name = 'profiles_id_fkey'
              AND table_name = 'profiles'
        ) THEN
            ALTER TABLE profiles
                ADD CONSTRAINT profiles_id_fkey
                FOREIGN KEY (id) REFERENCES auth.users(id) ON DELETE CASCADE;
        END IF;
    END IF;
END $$;

-- Wire tools.submitted_by -> profiles.id (declared as plain UUID in 001_init).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'tools_submitted_by_fkey'
          AND table_name = 'tools'
    ) THEN
        ALTER TABLE tools
            ADD CONSTRAINT tools_submitted_by_fkey
            FOREIGN KEY (submitted_by) REFERENCES profiles(id) ON DELETE SET NULL;
    END IF;
END $$;

-- Auto-update updated_at.
CREATE OR REPLACE FUNCTION profiles_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_profiles_set_updated_at
    BEFORE UPDATE ON profiles
    FOR EACH ROW
    EXECUTE FUNCTION profiles_set_updated_at();

-- ---------------------------------------------------------------------------
-- First-user-admin trigger
-- ---------------------------------------------------------------------------
-- The first profile inserted gets is_admin = true; subsequent ones get false.
-- Runs BEFORE INSERT so the row is written with the correct value in one step.
CREATE OR REPLACE FUNCTION set_first_user_admin()
RETURNS TRIGGER AS $$
DECLARE
    row_count INT;
BEGIN
    SELECT count(*) INTO row_count FROM profiles;
    IF row_count = 0 THEN
        NEW.is_admin := true;
    ELSE
        NEW.is_admin := COALESCE(NEW.is_admin, false);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_set_first_user_admin
    BEFORE INSERT ON profiles
    FOR EACH ROW
    WHEN (NEW.is_admin IS NULL OR NEW.is_admin = false)
    EXECUTE FUNCTION set_first_user_admin();

-- ---------------------------------------------------------------------------
-- siwx_sessions (server-side only — no RLS policies = client access blocked)
-- ---------------------------------------------------------------------------
CREATE TABLE siwx_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nonce TEXT NOT NULL UNIQUE,              -- 16-byte random, base64
    wallet_address TEXT NOT NULL,
    chain_id TEXT NOT NULL,                  -- '1' | 'solana' | ...
    message TEXT NOT NULL,                   -- full CAIP-122 message
    signature TEXT NOT NULL,                 -- hex signature
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expiration_time TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT false,
    used_at TIMESTAMPTZ,
    profile_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_siwx_sessions_nonce   ON siwx_sessions(nonce);
CREATE INDEX idx_siwx_sessions_wallet  ON siwx_sessions(wallet_address);
CREATE INDEX idx_siwx_sessions_used    ON siwx_sessions(used);

-- RLS enabled with NO policies => only service_role (server) can access.
ALTER TABLE siwx_sessions ENABLE ROW LEVEL SECURITY;

-- ---------------------------------------------------------------------------
-- profiles_public view (security_invoker = true)
-- ---------------------------------------------------------------------------
-- Exposes only anonymity-preserving fields. Never includes email, wallet
-- address, or GitHub username. security_invoker means the view runs with
-- the caller's privileges, so RLS on `profiles` still applies.
CREATE OR REPLACE VIEW profiles_public WITH (security_invoker = true) AS
    SELECT
        id,
        nickname,
        avatar_url,
        auth_method
    FROM profiles;

-- ---------------------------------------------------------------------------
-- RLS policies for profiles (SECURITY.md section 4.2)
-- ---------------------------------------------------------------------------
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;

-- Self read / insert / update
CREATE POLICY "Self read profile" ON profiles
    FOR SELECT TO authenticated
    USING ((select auth.uid()) = id);

CREATE POLICY "Self insert profile" ON profiles
    FOR INSERT TO authenticated
    WITH CHECK ((select auth.uid()) = id);

CREATE POLICY "Self update profile" ON profiles
    FOR UPDATE TO authenticated
    USING ((select auth.uid()) = id)
    WITH CHECK ((select auth.uid()) = id);

-- Admin read all profiles
CREATE POLICY "Admin read all profiles" ON profiles
    FOR SELECT TO authenticated
    USING (
        EXISTS (
            SELECT 1 FROM profiles p
            WHERE p.id = (select auth.uid()) AND p.is_admin = true
        )
    );

-- ---------------------------------------------------------------------------
-- Admin update policy for site_settings (depends on profiles.is_admin)
-- ---------------------------------------------------------------------------
CREATE POLICY "Admin update settings" ON site_settings
    FOR UPDATE TO authenticated
    USING (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (select auth.uid()) AND is_admin = true
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (select auth.uid()) AND is_admin = true
        )
    );
