-- 030_agent_sync.sql — Agent Sync tokens, device flow, bookmark source tracking.

-- ---------------------------------------------------------------------------
-- bookmarks: track save origin (web vs agent)
-- ---------------------------------------------------------------------------
ALTER TABLE bookmarks
  ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'web',
  ADD COLUMN IF NOT EXISTS source_client TEXT NULL;

ALTER TABLE bookmarks DROP CONSTRAINT IF EXISTS bookmarks_source_check;
ALTER TABLE bookmarks
  ADD CONSTRAINT bookmarks_source_check
  CHECK (source IN ('web', 'agent', 'import'));

ALTER TABLE bookmarks DROP CONSTRAINT IF EXISTS bookmarks_source_client_check;
ALTER TABLE bookmarks
  ADD CONSTRAINT bookmarks_source_client_check
  CHECK (
    source_client IS NULL
    OR source_client IN ('cursor', 'claude-code', 'windsurf', 'mcp', 'generic')
  );

-- Owner may update toolkit metadata (note/tags/source promotion).
CREATE POLICY "Self update bookmarks" ON bookmarks
  FOR UPDATE TO authenticated
  USING ((select auth.uid()) = user_id)
  WITH CHECK ((select auth.uid()) = user_id);

-- ---------------------------------------------------------------------------
-- agent_tokens — hashed PAT-style tokens for coding-tool MCP/REST
-- ---------------------------------------------------------------------------
CREATE TABLE agent_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    label TEXT NOT NULL DEFAULT 'Agent link',
    client TEXT NOT NULL DEFAULT 'generic'
      CHECK (client IN ('cursor', 'claude-code', 'windsurf', 'generic')),
    token_prefix TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scopes TEXT[] NOT NULL DEFAULT '{toolkit:write,blueprint:write}',
    default_blueprint_id UUID NULL REFERENCES blueprints(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ NULL,
    revoked_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_tokens_user_id ON agent_tokens(user_id);
CREATE INDEX idx_agent_tokens_user_active
  ON agent_tokens(user_id)
  WHERE revoked_at IS NULL;

ALTER TABLE agent_tokens ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Self read agent_tokens" ON agent_tokens
  FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);

CREATE POLICY "Self insert agent_tokens" ON agent_tokens
  FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self update agent_tokens" ON agent_tokens
  FOR UPDATE TO authenticated
  USING ((select auth.uid()) = user_id)
  WITH CHECK ((select auth.uid()) = user_id);

CREATE POLICY "Self delete agent_tokens" ON agent_tokens
  FOR DELETE TO authenticated
  USING ((select auth.uid()) = user_id);

-- ---------------------------------------------------------------------------
-- agent_device_sessions — OAuth-style device flow (no token copy)
-- ---------------------------------------------------------------------------
CREATE TABLE agent_device_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_code_hash TEXT NOT NULL UNIQUE,
    user_code TEXT NOT NULL UNIQUE,
    user_id UUID NULL REFERENCES profiles(id) ON DELETE CASCADE,
    agent_token_id UUID NULL REFERENCES agent_tokens(id) ON DELETE SET NULL,
    client TEXT NOT NULL DEFAULT 'generic'
      CHECK (client IN ('cursor', 'claude-code', 'windsurf', 'generic')),
    status TEXT NOT NULL DEFAULT 'pending'
      CHECK (status IN ('pending', 'approved', 'expired', 'consumed')),
    pending_token TEXT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    approved_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_device_sessions_user_code ON agent_device_sessions(user_code);
CREATE INDEX idx_agent_device_sessions_expires ON agent_device_sessions(expires_at);

ALTER TABLE agent_device_sessions ENABLE ROW LEVEL SECURITY;

-- Users may read their own approved sessions (device flow UI).
CREATE POLICY "Self read agent_device_sessions" ON agent_device_sessions
  FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id OR user_id IS NULL);

-- ---------------------------------------------------------------------------
-- agent_sync_log — idempotent audit trail (server INSERT only)
-- ---------------------------------------------------------------------------
CREATE TABLE agent_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    agent_token_id UUID NULL REFERENCES agent_tokens(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    tool_slug TEXT NULL,
    blueprint_id UUID NULL REFERENCES blueprints(id) ON DELETE SET NULL,
    idempotency_key TEXT NULL,
    status TEXT NOT NULL DEFAULT 'ok',
    detail JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX agent_sync_log_idempotency_unique
  ON agent_sync_log(user_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

CREATE INDEX idx_agent_sync_log_user_id ON agent_sync_log(user_id);

ALTER TABLE agent_sync_log ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Self read agent_sync_log" ON agent_sync_log
  FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);