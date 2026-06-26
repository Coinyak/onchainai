-- 009_featured_cards.sql — admin-managed featured carousel cards.

CREATE TABLE featured_cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    image_url TEXT NOT NULL,
    headline TEXT,
    subtitle TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_featured_cards_active_order
    ON featured_cards (is_active, sort_order ASC, created_at ASC);

CREATE INDEX idx_featured_cards_tool_id ON featured_cards (tool_id);

CREATE OR REPLACE FUNCTION featured_cards_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_featured_cards_set_updated_at
    BEFORE UPDATE ON featured_cards
    FOR EACH ROW
    EXECUTE FUNCTION featured_cards_set_updated_at();

ALTER TABLE featured_cards ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Public read active featured cards" ON featured_cards
    FOR SELECT
    USING (is_active = true);

CREATE POLICY "Admin insert featured cards" ON featured_cards
    FOR INSERT TO authenticated
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (SELECT auth.uid()) AND is_admin = true
        )
    );

CREATE POLICY "Admin update featured cards" ON featured_cards
    FOR UPDATE TO authenticated
    USING (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (SELECT auth.uid()) AND is_admin = true
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (SELECT auth.uid()) AND is_admin = true
        )
    );

CREATE POLICY "Admin delete featured cards" ON featured_cards
    FOR DELETE TO authenticated
    USING (
        EXISTS (
            SELECT 1 FROM profiles
            WHERE id = (SELECT auth.uid()) AND is_admin = true
        )
    );