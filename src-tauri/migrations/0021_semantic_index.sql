-- v2.0 slice 8: rebuildable semantic (vector) index.
-- Derived data: excluded from JSON export whitelist, safe to clear/rebuild.

CREATE TABLE IF NOT EXISTS semantic_index (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  embedding BLOB NOT NULL,
  model TEXT NOT NULL,
  dims INTEGER NOT NULL,
  indexed_at TEXT NOT NULL,
  PRIMARY KEY (entity_type, entity_id)
);
