-- Stage v1.1: templates, smart-list helpers, memory quick-insert.

ALTER TABLE memories ADD COLUMN quick_insert INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memories ADD COLUMN trigger_word TEXT;

CREATE TABLE IF NOT EXISTS item_templates (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('task', 'reminder', 'memory')),
  name TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_item_templates_active
  ON item_templates (deleted_at, kind, updated_at DESC);
