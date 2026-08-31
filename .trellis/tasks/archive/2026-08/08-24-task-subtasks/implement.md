# 实施计划：对 task 支持子任务

> 依 design.md 实施。顺序：后端数据层 → 后端服务层 → IPC → 前端 IPC 绑定 → 前端列表树形 → 前端详情面板 → 前端设置页 → 测试收尾 → 文档。

## 0. 前置

- [x] `git checkout -b feat/task-subtasks`（当前分支 `allan852/feat-sub-task`，确认基于 main）
- [x] 阅读 `.trellis/spec/frontend/` 与 `.trellis/spec/tauri/` 相关条目（component-guidelines / type-safety / quality-guidelines；tauri CSV 不需动）

## 1. 数据库迁移

- [x] 新建 `src-tauri/migrations/0022_task_subtasks.sql`（tasks 加 parent_id/child_order + 索引 + task_tree_expanded 表）
- [x] `src-tauri/src/infrastructure/db/mod.rs`：MIGRATIONS 追加 22；三处 schema_version 断言 21 → 22
- [x] 验证：`cd src-tauri && cargo test` 通过

## 2. 领域模型（domain/task.rs）

- [x] `Task` 加 `parent_id: Option<EntityId>`、`child_order: f64`
- [x] `CreateTaskInput` 加 `parent_id: Option<EntityId>`
- [x] `TaskQuery` 加 `parent_id: Option<EntityId>`
- [x] 新增 `TaskDeleteDisposition`（Cascade/Promote）、`TaskTreeExpanded`、`MAX_SUBTASK_DEPTH = 5`、`validate_parent_depth`

## 3. 设置项

- [x] `AppSettings` 加 `subtask_auto_complete_parent` / `subtask_cascade_children`（默认 true，serde default）
- [x] 设置测试：defaults roundtrip 覆盖新字段

## 4. 服务层（application/tasks.rs）

- [x] `TASK_ROW_SELECT` + `map_task_row` 追加 parent_id/child_order
- [x] 抽取 filter builder（query_tasks 复用）
- [x] `create_task` 支持 parent_id（校验、继承 list_id、child_order）
- [x] `query_tasks` 支持 parent_id 过滤
- [x] `complete_task` / `uncomplete_task` 完成聚合（级联子 + 向上自动完成，双向）
- [x] `archive_task` / `unarchive_task` 级联
- [x] `delete_task` 拒绝有子任务；新增 `delete_task_tree(id, disposition)`
- [x] 新增 `set_task_parent`（循环防护 / 深度 / 归巢 / 提升）
- [x] 新增 `reorder_subtasks`
- [x] 新增 `query_tree`
- [x] 新增 `tree_expanded_list` / `set_tree_expanded`；delete 路径清理展开状态
- [x] 单测（见 design 8.1），`cargo test` 全绿

## 5. IPC（commands/mod.rs）

- [x] 注册并实现新命令：task_query_tree / task_set_parent / task_reorder_subtasks / task_delete_tree / task_tree_expanded_list / task_set_tree_expanded
- [x] task_complete/task_uncomplete/task_archive/task_unarchive：对每个受影响任务 emit + 索引
- [x] 校验 `cargo test` 与 `cargo check`

## 6. 前端 IPC 绑定（src/ipc/client.ts）

- [x] Task / AppSettings / CreateTaskInput / TaskQuery 类型更新
- [x] 新增调用绑定 + TaskDeleteDisposition / TaskTreeExpanded 类型
- [x] `pnpm typecheck` 通过

## 7. 列表视图树形（TasksPage.tsx + TaskRow 相关）

- [x] 清单视图切 `taskQueryTree`，组装树（childrenMap + 排序）
- [x] 递归/扁平渲染：缩进、chevron 展开折叠（读写 taskSetTreeExpanded）
- [x] 父任务进度徽标（x/y）
- [x] 拖拽：间隙重排（reorderSubtasks）+ 归巢区（setParent）
- [x] 删除处置确认框（cascade / promote）
- [x] 空态/分页适配（树形视图隐藏 PagedListFooter 或调整）

## 8. 详情面板（TaskDetailPanel + 新 SubtaskSection）

- [x] `SubtaskSection.tsx`：列表 / 添加 / 勾选 / 改名 / 上下移 / 删除 / 点开详情 / 冻结
- [x] 接入 TaskDetailPanel（ChecklistSection 之前）
- [x] 详情删除按钮有子任务时走处置确认框
- [x] `onOpenTask` 注入（TasksPage → setSelectedId）

## 9. 设置页

- [x] SettingsPage 新增「子任务」区块两个开关
- [x] 文案对齐 empty-states 规范

## 10. 收尾验证

- [x] `cd src-tauri && cargo test`（全量）
- [x] `pnpm typecheck`
- [x] `pnpm lint`（若配置）
- [x] `pnpm test:unit`（若受影响）
- [x] 手动冒烟（pnpm tauri:dev 或 pnpm dev）：建子任务、勾选聚合、归巢、删除处置、展开记忆、重启保持

## 11. 文档与提交

- [x] docs/development-roadmap.md 更新「多层子任务」状态
- [x] 依 Phase 3.4 提交代码 + 规划工件

## 回滚点

- 迁移失败：删除 0022 迁移与 MIGRATIONS 条目，回退代码（数据库无损坏，0019-0021 不受影响）。
- 聚合逻辑 bug：可关闭两个设置开关退回旧行为（数据不变）。
