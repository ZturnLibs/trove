-- Stage 4: text clipboard history.

CREATE TABLE IF NOT EXISTS clipboard_items (
  id TEXT PRIMARY KEY NOT NULL,
  content TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  source_app TEXT,
  favorite INTEGER NOT NULL DEFAULT 0,
  use_count INTEGER NOT NULL DEFAULT 0,
  last_used_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_clipboard_active
  ON clipboard_items (deleted_at, favorite DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_clipboard_hash
  ON clipboard_items (content_hash, deleted_at);

CREATE INDEX IF NOT EXISTS idx_clipboard_expire
  ON clipboard_items (favorite, created_at);
