# CSV task import / export

## Scenario: append-only task CSV with preview and undo

### 1. Scope / Trigger

- Trigger: change CSV columns, preview/import/undo IPC, or migration `0018_import_batches.sql`.
- Product docs: `docs/csv-import.md`.
- Schema: `import_batches` table; db schema assertion must stay in sync (v1.4 → **18**).

### 2. Signatures

IPC (`commands/mod.rs`):

- `csv_export_tasks() -> String`
- `csv_preview_tasks(csv: String, mapping: Option<CsvFieldMapping>) -> Preview`
- `csv_import_tasks(csv, mapping) -> ImportResult`
- `csv_import_batches() -> Vec<ImportBatch>`
- `csv_undo_import(id) -> UndoResult`

Columns: `title,notes,status,priority,list,due_date,due_time,tags`.

### 3. Contracts

- CSV import **appends**. It does not replace the database (unlike JSON full restore).
- Preview must run before import: header mapping (including Chinese aliases), duplicate rows (title + due date), row errors, unmatched lists → Inbox.
- Any validation error → import refused (no partial insert).
- Mid-import failure rolls back tasks created in that batch.
- Undo deletes imported tasks only when `task.revision <= 2` (completing a task increments revision).

### 4. Validation & Error Matrix

| Condition | Error / behavior |
| --- | --- |
| Row has mapping/parse errors | preview lists them; import refuses the file |
| Duplicate title + due date | preview warning; import still allowed unless other errors |
| Unknown list name | task goes to Inbox |
| Undo after user edited/completed imported tasks (`revision > 2`) | those rows are skipped (not deleted) |
| JSON backup restore | out of scope for CSV commands |

### 5. Good / Base / Bad Cases

- Good: preview clean → import → `csv_undo_import` deletes untouched imported tasks
- Base: export current tasks, round-trip columns above (recurrence/reminders are omitted)
- Bad: treat CSV as a full backup or overwrite existing tasks

### 6. Tests Required

- Preview mapping aliases (Chinese headers)
- Import refuses when any row is invalid
- Import rollback on failure
- Undo respects `revision <= 2`
- Keep `tempdir` in scope for the whole `Database` life (opening by path reconnects; dropping the dir mid-test flakes)

### 7. Wrong vs Correct

#### Wrong

```rust
let dir = tempfile::tempdir().unwrap();
let db = Database::open(dir.path().join("t.db"));
drop(dir); // path is gone; later reconnects fail
```

#### Correct

```rust
let dir = tempfile::tempdir().unwrap();
let db = Database::open(dir.path().join("t.db"));
// keep `dir` alive until `db` (and any reopen-by-path) is done
```
