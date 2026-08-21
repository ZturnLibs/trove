-- v2.0 slice 1: AI suggestion ledger + memory sensitive flag.
-- Suggestions are derived data: excluded from JSON export whitelist, safe to clear.

CREATE TABLE IF NOT EXISTS ai_suggestions (
  id TEXT PRIMARY KEY NOT NULL,
  feature_type TEXT NOT NULL,
  source_entity_type TEXT NOT NULL,
  source_entity_id TEXT NOT NULL,
  payload TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  created_at TEXT NOT NULL,
  decided_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_ai_suggestions_feature_status
  ON ai_suggestions (feature_type, status);

CREATE INDEX IF NOT EXISTS idx_ai_suggestions_source
  ON ai_suggestions (source_entity_id);

ALTER TABLE memories ADD COLUMN sensitive INTEGER NOT NULL DEFAULT 0;
