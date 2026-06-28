-- 022_github_profile_id.sql — stable GitHub identity on profiles (avoids nickname collisions).

ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS github_id BIGINT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_github_id
    ON profiles (github_id)
    WHERE auth_method = 'github' AND github_id IS NOT NULL;