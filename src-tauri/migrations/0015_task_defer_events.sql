-- v1.3: lightweight defer/postpone stats for today smart sort (rebuildable auxiliary data).

CREATE TABLE IF NOT EXISTS task_defer_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('postpone', 'defer')),
  recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_defer_events_task
  ON task_defer_events (task_id);
