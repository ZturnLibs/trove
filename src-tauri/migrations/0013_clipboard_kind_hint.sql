-- v1.3: clipboard smart action kind hints (local rule classification).

ALTER TABLE clipboard_items ADD COLUMN kind_hint TEXT NOT NULL DEFAULT 'plain'
  CHECK (
    kind_hint IN ('plain', 'url', 'email', 'phone', 'date', 'code', 'error')
  );

CREATE INDEX IF NOT EXISTS idx_clipboard_kind_hint
  ON clipboard_items (kind_hint, deleted_at, created_at DESC);
