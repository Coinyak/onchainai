-- D2: Crawler schedule management (table is `sources` in this codebase).
ALTER TABLE sources
  ADD COLUMN IF NOT EXISTS schedule_minutes INT NOT NULL DEFAULT 360,
  ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT true;

-- Seed sensible defaults for known crawler sources.
UPDATE sources SET schedule_minutes = 60 WHERE name IN ('npm', 'github');
UPDATE sources SET schedule_minutes = 360 WHERE name = 'cryptoskill';
UPDATE sources SET schedule_minutes = 720 WHERE name = 'web3-mcp-hub';