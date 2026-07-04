-- 033_agent_device_sessions_rls_fix.sql — close device-flow RLS hole.
--
-- 030 allowed any authenticated user to SELECT pending sessions
-- (user_id IS NULL), exposing other users' user_code and enabling
-- session-fixation on the device flow. Server-side handlers use the
-- service connection, so the API never needed the NULL branch.
-- Approved sessions also expose pending_token; keep even self-reads
-- server-mediated (no PostgREST path).

DROP POLICY IF EXISTS "Self read agent_device_sessions" ON agent_device_sessions;

-- Owner may read only their own (post-approval) sessions, and never the
-- plaintext pending_token column via PostgREST column selection: revoke
-- column read from client roles entirely.
CREATE POLICY "Self read agent_device_sessions" ON agent_device_sessions
  FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);

REVOKE SELECT (pending_token, device_code_hash) ON agent_device_sessions
  FROM authenticated, anon;

-- Hygiene: expired sessions must not retain a plaintext token.
UPDATE agent_device_sessions
SET pending_token = NULL
WHERE pending_token IS NOT NULL
  AND expires_at < now();
