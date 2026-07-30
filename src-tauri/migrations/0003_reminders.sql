-- Stage 2: reminders, occurrences, recurring task series.

CREATE TABLE IF NOT EXISTS task_series (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  notes TEXT NOT NULL DEFAULT '',
  priority TEXT NOT NULL DEFAULT 'none'
    CHECK (priority IN ('none', 'low', 'medium', 'high')),
  list_id TEXT NOT NULL REFERENCES task_lists(id),
  recurrence_json TEXT NOT NULL,
  timezone TEXT NOT NULL,
  next_due_date TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  end_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

-- series_id is added without inline FK for SQLite alter compatibility.
ALTER TABLE tasks ADD COLUMN series_id TEXT;

CREATE TABLE IF NOT EXISTS reminders (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  notes TEXT NOT NULL DEFAULT '',
  task_id TEXT REFERENCES tasks(id),
  recurrence_json TEXT,
  timezone TEXT NOT NULL,
  next_fire_at TEXT NOT NULL,
  end_at TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_reminders_next_fire
  ON reminders (enabled, next_fire_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_reminders_task
  ON reminders (task_id, deleted_at);

CREATE TABLE IF NOT EXISTS reminder_occurrences (
  id TEXT PRIMARY KEY NOT NULL,
  reminder_id TEXT NOT NULL REFERENCES reminders(id),
  scheduled_at TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN (
      'pending',
      'scheduled',
      'actioned',
      'snoozed',
      'cancelled',
      'inferred_missed'
    )
  ),
  needs_schedule INTEGER NOT NULL DEFAULT 1,
  system_notification_id INTEGER,
  actioned_at TEXT,
  snooze_until TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_occ_unique
  ON reminder_occurrences (reminder_id, scheduled_at);

CREATE INDEX IF NOT EXISTS idx_reminder_occ_status
  ON reminder_occurrences (status, scheduled_at);
