-- 035_google_profile_id.sql — stable Google identity on profiles (mirrors 022 github_id).
--
-- Google sign-in keys a profile by the OIDC `sub` (subject) claim, which is
-- stable per Google account. auth_method extends the github|email|siwx set
-- with 'google'. Column is server-written only (no authenticated GRANT), so
-- the RLS column hardening in 023 needs no change.

ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS google_sub TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_google_sub
    ON profiles (google_sub)
    WHERE auth_method = 'google' AND google_sub IS NOT NULL;
