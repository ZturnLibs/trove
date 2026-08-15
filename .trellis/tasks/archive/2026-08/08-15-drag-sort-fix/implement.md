# 执行计划：拖拽排序跨清单语义修复 + @dnd-kit

## 目标

后端按清单分组重写 `sort_order`，前端引入 @dnd-kit 替换 HTML5 DnD，修复跨清单拖拽位置错乱。

## 前置

- 阅读顺序：prd.md → design.md。
- 依赖安装：`pnpm add @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities`，安装后立即 `pnpm typecheck` 验证 React 19 兼容。

## 步骤

### 后端（src-tauri/src/application/tasks.rs）

1. **改造 `reorder_tasks`**：
   - 先查询所有传入 id 的 `list_id`（一条 `SELECT id, list_id FROM tasks WHERE id IN (...)`）。
   - 按 `list_id` 分组，保留拖拽顺序。
   - 事务内每组按序写 `sort_order = 组内索引`，同时 `updated_at`/`revision` 更新。
   - 未在 ordered_ids 的任务不动。
2. **测试**：
   - 在 `tasks.rs` 对应测试模块（或 `src-tauri/tests/`）补：单清单重排、跨清单重排（验证两个清单各自顺序正确、互不污染）。
   - 运行 `cargo test`（在 src-tauri 目录）。

### 前端

3. **安装依赖并验证**：`pnpm add @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities` → `pnpm typecheck`。
4. **TaskRow.tsx**：移除原生 DnD props（`draggable`/`isDragging`/`isDropTarget`/`dropPosition`/`onDragStart/Over/Drop/End`）或重构。新增 `SortableTaskRow` 包装（`useSortable`），暴露 `setNodeRef`/`transform`/`transition`/`isDragging`。保持非拖拽调用点（InboxPage、TodayPage 其他分组）不受影响。
5. **TasksPage.tsx**：
   - 移除 `dragId`/`dropTarget`/`suppressClickUntilRef` 与 handleDrag* 函数、`applyDragReorder` 保留或重构为基于 @dnd-kit 的 `onDragEnd`。
   - 用 `DndContext` + `SortableContext` 包裹任务列表；`onDragEnd` 计算新顺序调用 `reorderMutation`。
   - `smart !== "none"` 时不包 SortableContext（保持不可拖拽）。
   - 保留上移/下移按钮（`moveSelected`）。
6. **TodayPage.tsx**：同样替换，仅 dueToday 分组包 `SortableContext`。移除上一轮临时实现（`dragId`/`dropTarget`/`handleTodaySelect` 的拖拽部分等）。逾期/已完成/提醒分组不包。
7. **DragOverlay / 插入指示**：拖拽时 DragOverlay 跟随；SortableContext 内置 transform 让位作为主要可视反馈。

### 验证门禁

8. `pnpm typecheck` 通过。
9. `pnpm test:unit` 通过。
10. `cargo test` 通过（src-tauri）。
11. 手动（开发环境已运行）：任务列表拖拽、今日任务分组跨清单拖拽、逾期/智能列表不可拖拽、双击重命名不受影响。

## 回滚点

- 后端：单独 commit `reorder_tasks` 改动，可单独 revert。
- 前端：单独 commit @dnd-kit 引入，可 revert 回 HTML5 DnD。
- 建议至少两个 commit：`fix(tasks): 按清单分组重写 sort_order` 与 `feat(tasks): 引入 @dnd-kit 拖拽排序`。
