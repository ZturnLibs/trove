-- v1.4 slice 5: CSV task import batches (preview + undo).

CREATE TABLE IF NOT EXISTS import_batches (
  id TEXT PRIMARY KEY NOT NULL,
  source TEXT NOT NULL,
  mapping_json TEXT NOT NULL,
  stats_json TEXT NOT NULL,
  task_ids_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  undone_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_import_batches_created
  ON import_batches (created_at DESC);
