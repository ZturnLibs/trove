-- Stage v1.2: image assets, OCR derived text, clipboard image rows.

CREATE TABLE IF NOT EXISTS assets (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('image')),
  content_hash TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  thumb_path TEXT,
  mime_type TEXT NOT NULL,
  byte_size INTEGER NOT NULL DEFAULT 0,
  width INTEGER,
  height INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_hash_active
  ON assets (content_hash) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS derived_texts (
  id TEXT PRIMARY KEY NOT NULL,
  asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('ocr')),
  text TEXT NOT NULL DEFAULT '',
  engine_version TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(asset_id, kind)
);

CREATE INDEX IF NOT EXISTS idx_derived_texts_asset
  ON derived_texts (asset_id);

ALTER TABLE clipboard_items ADD COLUMN kind TEXT NOT NULL DEFAULT 'text';
ALTER TABLE clipboard_items ADD COLUMN asset_id TEXT;
