-- v1.4 slice 3: rule automation (triggers, conditions, actions, run log).

CREATE TABLE IF NOT EXISTS automation_rules (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  definition_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS automation_runs (
  id TEXT PRIMARY KEY NOT NULL,
  rule_id TEXT NOT NULL,
  rule_name TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  status TEXT NOT NULL,
  actions_applied_json TEXT,
  error_summary TEXT,
  dry_run INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_automation_rules_active
  ON automation_rules (deleted_at, enabled, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_automation_runs_recent
  ON automation_runs (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_automation_runs_rule
  ON automation_runs (rule_id, created_at DESC);
