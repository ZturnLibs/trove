-- v2.1 subtasks: multi-level nested tasks + per-task tree expand state.
--
-- parent_id: self-referencing FK to tasks(id); NULL = top-level task.
--   Soft-delete semantics (deleted_at) mean app-level cascade, not FK cascade.
-- child_order: sibling ordering WITHIN a parent (REAL). Top-level ordering
--   keeps using list-scoped sort_order; nesting keeps the original list_id
--   (confirmed decision), so sibling order needs a parent-scoped key.

ALTER TABLE tasks ADD COLUMN parent_id TEXT REFERENCES tasks(id);
ALTER TABLE tasks ADD COLUMN child_order REAL NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_tasks_parent_active
  ON tasks (parent_id, deleted_at, child_order);

-- Tree expand/collapse state (database-persisted, confirmed decision B).
-- Semantics: row present with expanded=0 => collapsed; missing row => expanded.
CREATE TABLE IF NOT EXISTS task_tree_expanded (
  task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  expanded INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL
);
