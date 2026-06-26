-- 008_operator_harness.sql — Hermes operator harness tasks and agent proposals.
--
-- operator_tasks: internal admin work items surfaced to operators and agents.
-- agent_action_proposals: structured Hermes recommendations awaiting human approval.

-- ---------------------------------------------------------------------------
-- operator_tasks
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS operator_tasks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',
  subject_tool_id UUID REFERENCES tools(id) ON DELETE SET NULL,
  priority INT NOT NULL DEFAULT 0,
  summary TEXT NOT NULL,
  payload JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'operator_tasks_status_check' AND conrelid = 'operator_tasks'::regclass
  ) THEN
    ALTER TABLE operator_tasks
      ADD CONSTRAINT operator_tasks_status_check
      CHECK (status IN ('open', 'in_progress', 'done', 'cancelled'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_operator_tasks_status ON operator_tasks(status);
CREATE INDEX IF NOT EXISTS idx_operator_tasks_subject_tool_id ON operator_tasks(subject_tool_id);
CREATE INDEX IF NOT EXISTS idx_operator_tasks_created_at ON operator_tasks(created_at DESC);

ALTER TABLE operator_tasks ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read operator tasks" ON operator_tasks;
CREATE POLICY "Admin read operator tasks" ON operator_tasks
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write operator tasks" ON operator_tasks;
CREATE POLICY "Admin write operator tasks" ON operator_tasks
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

-- ---------------------------------------------------------------------------
-- agent_action_proposals
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS agent_action_proposals (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_name TEXT NOT NULL,
  action_type TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'proposed',
  subject_tool_id UUID REFERENCES tools(id) ON DELETE SET NULL,
  proposal JSONB NOT NULL,
  evidence JSONB NOT NULL DEFAULT '[]',
  approved_by UUID REFERENCES profiles(id) ON DELETE SET NULL,
  executed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'agent_action_proposals_status_check'
      AND conrelid = 'agent_action_proposals'::regclass
  ) THEN
    ALTER TABLE agent_action_proposals
      ADD CONSTRAINT agent_action_proposals_status_check
      CHECK (status IN ('proposed', 'approved', 'rejected', 'executed', 'cancelled'));
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'agent_action_proposals_action_type_check'
      AND conrelid = 'agent_action_proposals'::regclass
  ) THEN
    ALTER TABLE agent_action_proposals
      ADD CONSTRAINT agent_action_proposals_action_type_check
      CHECK (action_type IN (
        'approve', 'reject', 'needs_info', 'quarantine', 'outreach',
        'deploy', 'cleanup', 'mark_official', 'mark_verified', 'auth_change'
      ));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_agent_action_proposals_status ON agent_action_proposals(status);
CREATE INDEX IF NOT EXISTS idx_agent_action_proposals_subject_tool_id ON agent_action_proposals(subject_tool_id);
CREATE INDEX IF NOT EXISTS idx_agent_action_proposals_created_at ON agent_action_proposals(created_at DESC);

ALTER TABLE agent_action_proposals ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Admin read agent proposals" ON agent_action_proposals;
CREATE POLICY "Admin read agent proposals" ON agent_action_proposals
  FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );

DROP POLICY IF EXISTS "Admin write agent proposals" ON agent_action_proposals;
CREATE POLICY "Admin write agent proposals" ON agent_action_proposals
  FOR ALL TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  )
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM profiles
      WHERE id = (select auth.uid()) AND is_admin = true
    )
  );