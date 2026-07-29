-- Stage 0 baseline schema.
-- Conventions: UUID primary keys, created_at/updated_at (ISO-8601 UTC),
-- revision (monotonic), deleted_at for soft delete.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY NOT NULL,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY NOT NULL,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- Smoke-test entity used in stage 0 to verify persistence across restarts.
CREATE TABLE IF NOT EXISTS smoke_notes (
  id TEXT PRIMARY KEY NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_smoke_notes_active
  ON smoke_notes (deleted_at, updated_at DESC);
