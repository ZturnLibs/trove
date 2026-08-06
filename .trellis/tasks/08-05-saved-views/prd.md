# 自定义视图 SavedView（saved-views）

## Goal

任务页支持把当前筛选（清单/状态/优先级/标签）保存为命名视图，一键复用，兑现 v1.1「保存当前筛选为自定义视图」承诺。

## 背景（勘察确认）

- TasksPage 筛选状态：`listId` / `status` / `priority` / `tagId` / `smart`（TasksPage.tsx:36-39），queryKey `["tasks","list",listId,status,priority,smart,tagId]`。
- 无 SavedView 表/命令/UI（全库 `rg SavedView` 无命中）。
- 数据库迁移：当前到 0008，`db/mod.rs` `MIGRATIONS` 注册顺序迁移，`schema_version == 8` 断言在 db/mod.rs 测试。

## Requirements

- R1 新增迁移 `0009_saved_views.sql`：`saved_views(id TEXT PK, name TEXT NOT NULL, filter_json TEXT NOT NULL, created_at, updated_at, revision, deleted_at)`；`db/mod.rs` 注册 `(9, …)` 并更新 schema_version 断言为 9。
- R2 后端新增 `SavedViewService`（`application/saved_views.rs`）：`create(input)/list()/delete(id)`；命令 `saved_view_create/list/delete` + lib.rs 注册。
- R3 client.ts 新增 `savedViewCreate({ name, filter })` / `savedViewList()` / `savedViewDelete(id)` 封装与类型。
- R4 TasksPage：
  - actions 区新增「保存视图」按钮：点击展开名称输入（内联小输入框 + 确认），保存当前 `{ listId, status, priority, tagId, smart }`；
  - 新增「视图」下拉（有保存视图时显示）：含「无」+ 各视图；选中后应用其 filter（设置对应 state，`smart` 保留保存时的值）；
  - 删除：视图下拉旁小 ✕ 或选中视图后提供删除（用 ConfirmButton）。
- R5 保存/删除后失效 `["saved-views"]`；应用视图后 queryKey 自动重查。

## Acceptance Criteria

- [ ] AC1 迁移 0009 生效，`schema_version == 9`，cargo test 通过。
- [ ] AC2 可保存当前筛选为命名视图，列表出现；应用后筛选生效。
- [ ] AC3 可删除已保存视图（ConfirmButton 确认）。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 中-大复杂度任务：PRD + design。改动为迁移 + 新服务/命令 + 前端。
- MVP：视图 = 任务页筛选预设（不含今日页/智能列表快捷组合之外的复杂条件）。
