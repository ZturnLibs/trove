# 技术设计：自定义视图 SavedView

## 1. 迁移 0009

`migrations/0009_saved_views.sql`：

```sql
CREATE TABLE IF NOT EXISTS saved_views (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  filter_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 0,
  deleted_at TEXT
);
```

`db/mod.rs` `MIGRATIONS` 追加 `(9, include_str!("../../../migrations/0009_saved_views.sql"))`；两个迁移测试断言 `schema_version == 8` → `9`。

## 2. SavedViewService（application/saved_views.rs）

```rust
pub struct SavedView { id, name, filter_json: serde_json::Value, created_at, updated_at, revision }
pub struct SavedViewService { db, clock }  // 仿 TemplateService

pub fn create(&self, name: String, filter: serde_json::Value) -> Result<SavedView>
pub fn list(&self) -> Result<Vec<SavedView>>          // 按 updated_at DESC
pub fn delete(&self, id: EntityId) -> Result<()>      // 软删 deleted_at
```

`mod.rs` 注册模块；`app_state.rs` 注入 `SavedViewService`。

## 3. 命令 + client

commands/mod.rs：`saved_view_create(input: { name, filter: Value })` / `saved_view_list()` / `saved_view_delete(id)`；lib.rs 注册。

client.ts：

```ts
export type SavedView = { id: string; name: string; filter: Record<string, unknown>; createdAt: string; updatedAt: string; revision: number };
savedViewCreate: (input: { name: string; filter: Record<string, unknown> }) => invoke<SavedView>("saved_view_create", { input }),
savedViewList: () => invoke<SavedView[]>("saved_view_list"),
savedViewDelete: (id: string) => invoke<void>("saved_view_delete", { id }),
```

## 4. TasksPage UI

- `savedViewsQuery`（key `["saved-views"]`）。
- 「保存视图」按钮 → 展开名称输入（内联），确认后 `savedViewCreate({ name, filter: { listId, status, priority, tagId, smart } })`；空名禁用。
- 「视图」下拉：`<option value="">无</option>` + 各视图；onChange 应用：`setListId(filter.listId ?? "all")` / `setStatus(...)` / `setPriority(...)` / `setTagId(...)` / `setSmart(...)`。
- 删除：下拉旁「管理」或每个视图行删除，用 ConfirmButton；本 MVP 用「视图」下拉 + 删除按钮（选中某视图时出现「删除视图」ConfirmButton）。

## 5. 边界

- 视图仅任务页筛选预设；不包含「新清单」等局部 UI 态。
- filter JSON 由前端直接构造（{ listId, status, priority, tagId, smart }），后端仅存读不解释。
