# Tauri / Rust Guidelines

Contracts for the `src-tauri` crate. Read the listed files before changing the matching area.

## Pre-Development Checklist

- [ ] Adding `src/bin/*` or a second Cargo binary → [extra-binaries-and-release.md](./extra-binaries-and-release.md)
- [ ] Changing `WorkbenchAction` / `ActionOutcome` / IPC `workbench_action_dispatch` → [workbench-action.md](./workbench-action.md)
- [ ] Changing task CSV import/export/undo or migration `0018` → [csv-tasks.md](./csv-tasks.md)
- [ ] Changing `tests/e2e/**`、CI `e2e` job 或 app 启动 env/data-dir 链路 → [e2e-webdriver.md](./e2e-webdriver.md)
- [ ] Bumping app version or cutting a GitHub Release → [extra-binaries-and-release.md](./extra-binaries-and-release.md)

## Quality Check

- [ ] `cargo test --lib` in `src-tauri`
- [ ] Schema assertion for the latest migration number is bumped in **all five** places: three in `infrastructure/db/mod.rs` (health/idempotent/backup tests) **and two in `application/backup.rs`** (restore tests). Missing the backup.rs pair surfaces only as a failing `restore_old_schema_backup_then_migrates` test.
- [ ] `tauri build` (or CI `release` workflow) still finds the **app** binary after extra bins exist
- [ ] Database tests keep the `tempdir` alive for the whole `Database` lifetime

## Pitfalls (learned)

- **Nullable column reads**: a column that can be NULL (e.g. `tasks.parent_id`) must be
  read with `row.get::<_, Option<String>>(0)` — plain `row.get(0)` fails with
  `Invalid column type Null`. This also means dropping the `.optional()` wrapper
  when the closure already returns `Option`.
- **Partial settings deserialization**: when a service reads only a few fields from
  the `app.settings` JSON blob with a private `#[derive(Deserialize)]` struct, add
  `#[serde(rename_all = "camelCase")]` on that struct too. Without it, camelCase
  keys (e.g. `subtaskAutoCompleteParent`) silently fall back to `default`, making
  toggles appear stuck "on".
- **Joined filter dimensions must exclude soft-deleted rows**: when adding a filter
  that resolves through a join (e.g. tasks → list → group) in `build_task_filters`,
  require `deleted_at IS NULL` on **every** joined table — the task row is live, but a
  soft-deleted list/group silently resurrects matches. Found in task-list-groups;
  regression test: `group_filter_excludes_soft_deleted_list_and_group`.
- **Clearing an optional relation needs an explicit flag, not a bare `Option`**: in
  `task_list_update`, `group_id: Option<EntityId>` + `clear_group: Option<bool>` form a
  three-state input — absent = don't change, `group_id = Some(x)` = assign, and
  "unassign" (set to NULL) is only expressible via `clear_group = Some(true)`, which
  takes priority. A bare `Option` cannot distinguish "unchanged" from "set to NULL";
  any new command that can clear an optional FK must ship this pair (see
  `commands/mod.rs` `task_list_update`).
- **Subtask aggregation**: `complete_task` / `uncomplete_task` / `archive_task` /
  `unarchive_task` cascade and aggregate inside a single transaction; series spawn
  happens only for the explicitly targeted task. Both aggregation toggles live in
  `AppSettings` (`subtask_auto_complete_parent`, `subtask_cascade_children`),
  default true.

## Guidelines Index

| Guide | Description |
| --- | --- |
| [Extra binaries and release](./extra-binaries-and-release.md) | `default-run`, universal lipo, `beforeBundleCommand` cwd |
| [Workbench action](./workbench-action.md) | Serde tag, confirmation gate, IPC |
| [CSV tasks](./csv-tasks.md) | Preview/import/undo contracts |
