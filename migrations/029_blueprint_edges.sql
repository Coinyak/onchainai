-- 029_blueprint_edges.sql — Stack Blueprint v2: node-to-node connection edges.

ALTER TABLE blueprints
    ADD COLUMN IF NOT EXISTS edges JSONB NOT NULL DEFAULT '[]';