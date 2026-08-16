-- v1.2 capture remaining: file references (bookmark + path hint, no file copy).

CREATE TABLE IF NOT EXISTS file_references (
  id TEXT PRIMARY KEY NOT NULL,
  display_name TEXT NOT NULL,
  path_hint TEXT NOT NULL,
  mime_type TEXT,
  byte_size INTEGER,
  bookmark BLOB,
  accessible INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_file_references_active
  ON file_references (deleted_at, updated_at DESC);
