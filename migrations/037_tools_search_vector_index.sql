-- Expand GIN FTS index to match application TOOL_SEARCH_VECTOR (name, description, slug, repo_url).
-- Keeps idx_tools_search name; expression must mirror src/server/tool_search.rs tool_search_vector!().

DROP INDEX IF EXISTS idx_tools_search;

CREATE INDEX idx_tools_search ON tools USING GIN (
    (
        setweight(to_tsvector('english', coalesce(name, '')), 'A')
        || setweight(to_tsvector('english', coalesce(description, '')), 'B')
        || setweight(to_tsvector('english', coalesce(slug, '')), 'A')
        || setweight(to_tsvector('english', coalesce(repo_url, '')), 'C')
    )
);