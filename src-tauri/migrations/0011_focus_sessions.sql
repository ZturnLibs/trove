-- Focus sessions and daily wrap runs (v1.3 focus + daily wrap).

CREATE TABLE IF NOT EXISTS focus_sessions (
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  planned_minutes INTEGER,
  outcome TEXT NOT NULL DEFAULT 'in_progress'
    CHECK (outcome IN ('in_progress', 'completed', 'kept_todo', 'abandoned')),
  progress_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_focus_sessions_task
  ON focus_sessions (task_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_focus_sessions_active
  ON focus_sessions (outcome)
  WHERE outcome = 'in_progress';

CREATE TABLE IF NOT EXISTS daily_wrap_runs (
  id TEXT PRIMARY KEY NOT NULL,
  wrap_date TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  steps_completed INTEGER NOT NULL DEFAULT 0,
  summary_json TEXT,
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_daily_wrap_date
  ON daily_wrap_runs (wrap_date)
  WHERE completed_at IS NOT NULL;
