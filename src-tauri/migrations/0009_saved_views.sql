-- Stage v1.2: saved task-page filter presets (自定义视图).

CREATE TABLE IF NOT EXISTS saved_views (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  filter_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 0,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_saved_views_active
  ON saved_views (deleted_at, updated_at DESC);
