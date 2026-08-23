-- v2.0 slice 6: one-level task checklist items (foundation for AI task split).
-- Soft-deleted alongside their task; no cascade needed beyond app-level purge.

CREATE TABLE IF NOT EXISTS task_checklist_items (
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL,
  content TEXT NOT NULL,
  checked INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_checklist_task
  ON task_checklist_items (task_id, deleted_at);
