-- v1.3 GTD workflow: defer display, waiting, daily focus.

ALTER TABLE tasks ADD COLUMN workflow_state TEXT NOT NULL DEFAULT 'active'
  CHECK (workflow_state IN ('active', 'waiting'));
ALTER TABLE tasks ADD COLUMN available_at TEXT;
ALTER TABLE tasks ADD COLUMN waiting_for TEXT;
ALTER TABLE tasks ADD COLUMN follow_up_date TEXT;

CREATE INDEX IF NOT EXISTS idx_tasks_available_active
  ON tasks (available_at, status, deleted_at)
  WHERE deleted_at IS NULL AND status = 'todo';

CREATE INDEX IF NOT EXISTS idx_tasks_waiting_followup
  ON tasks (follow_up_date, workflow_state, deleted_at)
  WHERE deleted_at IS NULL AND workflow_state = 'waiting';

CREATE TABLE IF NOT EXISTS daily_focus (
  focus_date TEXT NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  sort_order REAL NOT NULL DEFAULT 0,
  added_at TEXT NOT NULL,
  carried_from_date TEXT,
  PRIMARY KEY (focus_date, task_id)
);

CREATE INDEX IF NOT EXISTS idx_daily_focus_date_order
  ON daily_focus (focus_date, sort_order);
