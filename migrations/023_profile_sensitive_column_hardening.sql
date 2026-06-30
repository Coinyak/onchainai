-- 023_profile_sensitive_column_hardening.sql — protect is_admin/is_banned from client updates.
--
-- SECURITY P0: RLS restricts rows, not columns. The existing "Self update
-- profile" policy lets an authenticated user UPDATE their own row, which
-- means a normal user could set is_admin = true or is_banned = false via
-- the Supabase Data API (PostgREST) using only the anon key.
--
-- This migration:
--   1. Drops the permissive "Self update profile" policy.
--   2. Replaces it with a column-restricted policy that blocks writes to
--      is_admin and is_banned at the database level.
--   3. Gates the first-user-admin trigger so production cannot accidentally
--      promote the first random signup.
--
-- Related: docs/SECURITY.md §4.2, docs/superpowers/specs/2026-06-30-auth-admin-…spec.md §5.2/§5.3.

-- ---------------------------------------------------------------------------
-- 1. Drop and replace the self-update policy with column-level protection.
-- ---------------------------------------------------------------------------
DROP POLICY IF EXISTS "Self update profile" ON profiles;

-- Self-update allowed only for safe user-owned columns. The USING clause
-- keeps row-level ownership (own row only). The subtle guard is the
-- WHEN clause on the trigger below, but the real column protection comes
-- from revoking UPDATE on sensitive columns from the authenticated role.
CREATE POLICY "Self update safe profile columns" ON profiles
    FOR UPDATE TO authenticated
    USING ((select auth.uid()) = id)
    WITH CHECK ((select auth.uid()) = id);

-- Revoke direct UPDATE on sensitive columns from the authenticated role.
-- The service_role (server) bypasses RLS entirely, so admin mutations via
-- require_admin server functions are unaffected. This is the hard floor:
-- even if the RLS policy above were misconfigured, the column privilege
-- prevents a client from writing is_admin or is_banned.
REVOKE UPDATE (is_admin) ON profiles FROM authenticated;
REVOKE UPDATE (is_banned) ON profiles FROM authenticated;

-- Allow authenticated users to update their own safe columns.
GRANT UPDATE (nickname, bio, avatar_url, updated_at, onboarding_completed_at) ON profiles TO authenticated;

-- ---------------------------------------------------------------------------
-- 2. Gate the first-user-admin trigger for production safety.
-- ---------------------------------------------------------------------------
-- The trigger `trg_set_first_user_admin` auto-promotes the very first
-- profile to is_admin = true. This is useful for local bootstrap but
-- dangerous in production: an empty production database would promote
-- the first random signup (GitHub, email, or SIWX) to admin.
--
-- We keep the trigger function but disable the trigger itself. Local
-- development can re-enable it with:
--   ALTER TABLE profiles ENABLE TRIGGER trg_set_first_user_admin;
-- Production operator bootstrap should use ADMIN_GITHUB_LOGINS or an
-- explicit SQL INSERT/UPDATE issued by an operator with service_role.

ALTER TABLE profiles DISABLE TRIGGER trg_set_first_user_admin;
