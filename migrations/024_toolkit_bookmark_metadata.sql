-- 024_toolkit_bookmark_metadata.sql — saved toolkit notes, tags, and update ordering.

ALTER TABLE bookmarks
  ADD COLUMN IF NOT EXISTS note TEXT,
  ADD COLUMN IF NOT EXISTS tags TEXT[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE INDEX IF NOT EXISTS idx_bookmarks_user_updated_at
  ON bookmarks(user_id, updated_at DESC);

CREATE OR REPLACE FUNCTION bookmarks_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_bookmarks_set_updated_at ON bookmarks;
CREATE TRIGGER trg_bookmarks_set_updated_at
    BEFORE UPDATE ON bookmarks
    FOR EACH ROW
    EXECUTE FUNCTION bookmarks_set_updated_at();
