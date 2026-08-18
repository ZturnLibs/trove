# WorkbenchAction contracts

## Scenario: shared action enum for UI, URL, CLI, automation

### 1. Scope / Trigger

- Trigger: add/rename a `WorkbenchAction` / `ActionOutcome` variant, or change IPC `workbench_action_dispatch`.
- Product docs: `docs/action-layer.md`, `docs/cli.md`, `docs/url-scheme.md`.

### 2. Signatures

- Domain: `src-tauri/src/domain/workbench_action.rs`
  - `#[serde(tag = "action", rename_all = "camelCase")]` on `WorkbenchAction`
  - `#[serde(tag = "outcome", rename_all = "camelCase")]` on `ActionOutcome`
- Dispatch: `application/workbench_actions.rs` → `dispatch(...)`
- IPC: `workbench_action_dispatch(action, options)`
- CLI protocol prefix: `trove-action:` (JSON after the prefix). CLI does not open SQLite; it forwards to the running app.

### 3. Contracts

- Serde **tag field is `action`**, not `kind`. Several variants already have a `kind` field (`CreatePreview`, create payloads). Using `tag = "kind"` collides and fails to deserialize.
- Mutating actions from UrlScheme / CLI require `confirmed: true` (or `--yes` on CLI, which sets that flag). Unconfirmed writes return `Rejected`.
- `dry_run: true` returns a description and does not persist.
- `trove://create` stays preview-first (same as v1.3). Silent create is CLI `--yes` / explicit `confirmed`.

### 4. Validation & Error Matrix

| Condition | Error / outcome |
| --- | --- |
| Write action, `confirmed != true` | `Rejected` / validation: needs `confirmed=true` |
| JSON tagged with `"kind": "navigate"` | deserialize fail (tag is `action`) |
| CLI write without `--yes` | opens in-app confirm (or create preview), does not persist |
| App not running | CLI starts/activates the app via the `trove-action:` protocol; it must not open the DB itself |

### 5. Good / Base / Bad Cases

- Good: `{ "action": "navigate", "path": "/today" }`
- Base: URL scheme parses to `WorkbenchAction` then `dispatch`
- Bad: `{ "kind": "navigate", "path": "/today" }` as the wire format

### 6. Tests Required

- Round-trip serde for a variant that also has a `kind` field (`CreatePreview`)
- Confirmation gate: unconfirmed `CreateTask` / `CompleteTask` is rejected
- `cargo test --lib`

### 7. Wrong vs Correct

#### Wrong

```rust
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WorkbenchAction { /* CreatePreview { kind, .. } */ }
```

#### Correct

```rust
#[serde(tag = "action", rename_all = "camelCase")]
pub enum WorkbenchAction {
    Navigate { path: String },
    CreatePreview { kind: CreateKind, /* ... */ },
}
```
