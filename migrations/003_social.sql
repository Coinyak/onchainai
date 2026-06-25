-- 003_social.sql — OnchainAI social features schema.
--
-- Creates `comments` (1-level threading via parent_id), `upvotes` (unique
-- per comment+user), and `bookmarks` (unique per tool+user), with the RLS
-- policies from SECURITY.md section 4.2.

-- ---------------------------------------------------------------------------
-- comments (public read, auth write, 1-level threading)
-- ---------------------------------------------------------------------------
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES comments(id) ON DELETE CASCADE, -- NULL = top-level
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    content TEXT NOT NULL,                   -- 1-2000 chars (validated app-side)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_comments_tool_id    ON comments(tool_id);
CREATE INDEX idx_comments_parent_id  ON comments(parent_id);
CREATE INDEX idx_comments_user_id    ON comments(user_id);
CREATE INDEX idx_comments_created_at ON comments(created_at);

CREATE OR REPLACE FUNCTION comments_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_comments_set_updated_at
    BEFORE UPDATE ON comments
    FOR EACH ROW
    EXECUTE FUNCTION comments_set_updated_at();

ALTER TABLE comments ENABLE ROW LEVEL SECURITY;

-- Public read
CREATE POLICY "Public read comments" ON comments
    FOR SELECT TO anon, authenticated USING (true);

-- Authenticated users insert (must own the row)
CREATE POLICY "Auth insert comments" ON comments
    FOR INSERT TO authenticated
    WITH CHECK ((select auth.uid()) = user_id);

-- Author update / delete
CREATE POLICY "Author update comments" ON comments
    FOR UPDATE TO authenticated
    USING ((select auth.uid()) = user_id)
    WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Author delete comments" ON comments
    FOR DELETE TO authenticated
    USING ((select auth.uid()) = user_id);

-- ---------------------------------------------------------------------------
-- upvotes (unique per comment+user, owner-only access)
-- ---------------------------------------------------------------------------
CREATE TABLE upvotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Unique constraint: one upvote per (comment, user).
CREATE UNIQUE INDEX upvotes_comment_user_unique
    ON upvotes(comment_id, user_id);

CREATE INDEX idx_upvotes_comment_id ON upvotes(comment_id);
CREATE INDEX idx_upvotes_user_id    ON upvotes(user_id);

ALTER TABLE upvotes ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Self read upvotes" ON upvotes
    FOR SELECT TO authenticated
    USING ((select auth.uid()) = user_id);

CREATE POLICY "Self insert upvotes" ON upvotes
    FOR INSERT TO authenticated
    WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self delete upvotes" ON upvotes
    FOR DELETE TO authenticated
    USING ((select auth.uid()) = user_id);

-- ---------------------------------------------------------------------------
-- bookmarks (unique per tool+user, owner-only access)
-- ---------------------------------------------------------------------------
CREATE TABLE bookmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Unique constraint: one bookmark per (tool, user).
CREATE UNIQUE INDEX bookmarks_tool_user_unique
    ON bookmarks(tool_id, user_id);

CREATE INDEX idx_bookmarks_tool_id ON bookmarks(tool_id);
CREATE INDEX idx_bookmarks_user_id ON bookmarks(user_id);

ALTER TABLE bookmarks ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Self read bookmarks" ON bookmarks
    FOR SELECT TO authenticated
    USING ((select auth.uid()) = user_id);

CREATE POLICY "Self insert bookmarks" ON bookmarks
    FOR INSERT TO authenticated
    WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self delete bookmarks" ON bookmarks
    FOR DELETE TO authenticated
    USING ((select auth.uid()) = user_id);
