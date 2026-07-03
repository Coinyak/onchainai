-- 027_blueprints.sql — Stack Blueprint canvas storage (mirrors bookmarks.user_id pattern).

CREATE TABLE blueprints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT 'Untitled blueprint',
    nodes JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_blueprints_user ON blueprints(user_id, updated_at DESC);

CREATE OR REPLACE FUNCTION blueprints_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_blueprints_set_updated_at
    BEFORE UPDATE ON blueprints
    FOR EACH ROW
    EXECUTE FUNCTION blueprints_set_updated_at();

ALTER TABLE blueprints ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Self read blueprints" ON blueprints
    FOR SELECT TO authenticated
    USING ((select auth.uid()) = user_id);

CREATE POLICY "Self insert blueprints" ON blueprints
    FOR INSERT TO authenticated
    WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self update blueprints" ON blueprints
    FOR UPDATE TO authenticated
    USING ((select auth.uid()) = user_id)
    WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self delete blueprints" ON blueprints
    FOR DELETE TO authenticated
    USING ((select auth.uid()) = user_id);