# 任务拖拽排序：修复跨清单顺序语义 + @dnd-kit 拖拽

## Goal

修复任务拖拽排序「释放后位置不对」的问题，并引入 @dnd-kit 提供更可靠的拖拽体验与目标位置可视化。

## Background

- 现状：任务列表（`TasksPage`）与今日任务分组（`TodayPage`）均用原生 HTML5 DnD 实现拖拽，调用 `ipc.taskReorder(orderedIds)`。
- 问题根因：`sort_order` 在数据库中按**清单独立**（创建时 `tasks.rs:324` 按 `list_id` 取 `MAX(sort_order)+1`），但 `reorder_tasks`（`tasks.rs:689`）把传入的 id 直接写成**全局连续** `0..n-1`。任务列表页（单清单）无影响；今日视图**跨清单混排**时，重排会把各清单的序号写串：
  - 刷新后位置错乱（sort_order 冲突）
  - 污染任务列表页里这些任务在各清单内的原有顺序
- 用户反馈：拖拽释放后位置不对，且希望拖拽过程中明确显示目标插入位置。

## Requirements

1. **后端语义修复**：`reorder_tasks` 按清单分组重写 `sort_order`——对传入的 orderedIds 按 `list_id` 分组，组内按拖拽顺序分配连续序号；不污染其他清单。
2. **前端引入 @dnd-kit**：替换 `TasksPage` 与 `TodayPage` 的原生 HTML5 DnD 实现，使用 `@dnd-kit/core` + `@dnd-kit/sortable`。
3. **拖拽过程中显示目标位置**：使用 DragOverlay / 插入指示，明确预览插入点。
4. 保持键盘优先交互与「上移/下移」按钮不受影响。
5. 今日视图仅「今日任务」分组启用拖拽（与现有范围一致）。

## Constraints

- 后端改动限 `reorder_tasks`（及必要的事务/测试），不改数据模型与排序 SQL 的其他部分。
- 前端改动限 `TasksPage.tsx` / `TodayPage.tsx` / `TaskRow.tsx`。
- `TaskRow` 保持向后兼容（其他调用点：InboxPage、TodayPage 逾期/已完成分组不传拖拽 props）。
- 新依赖仅 `@dnd-kit/core`、`@dnd-kit/sortable`、`@dnd-kit/utilities`。

## Acceptance Criteria

- [ ] 任务列表页（单清单）拖拽后刷新位置正确
- [ ] 今日任务分组跨清单拖拽后刷新位置正确，且不污染各清单内顺序（回归：任务列表页各清单顺序不变）
- [ ] 拖拽过程中显示清晰的插入位置指示
- [ ] 智能列表 / 逾期 / 已完成 / 提醒分组不可拖拽
- [ ] 键盘选中、Enter/Space、双击重命名不受影响
- [ ] `pnpm typecheck` 与 `pnpm test:unit` 通过；后端 `cargo test` 通过

## Notes

- 复杂任务：含后端语义修复 + 前端库引入，需 `design.md` + `implement.md`。
- 参考：`docs/ui-layout-interaction.md:269`「列表支持拖拽排序（仅当前视图排序值）」。
