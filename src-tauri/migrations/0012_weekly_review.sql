CREATE TABLE review_sessions (
  id TEXT PRIMARY KEY NOT NULL,
  review_type TEXT NOT NULL DEFAULT 'weekly'
    CHECK (review_type IN ('weekly')),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  summary_json TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_review_sessions_type_completed
  ON review_sessions (review_type, completed_at DESC);
