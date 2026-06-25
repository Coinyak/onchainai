-- 004_onboarding.sql — profile onboarding completion gate.

ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS onboarding_completed_at TIMESTAMPTZ;

-- Existing accounts skip the first-login onboarding flow.
UPDATE profiles
SET onboarding_completed_at = COALESCE(onboarding_completed_at, created_at)
WHERE onboarding_completed_at IS NULL;