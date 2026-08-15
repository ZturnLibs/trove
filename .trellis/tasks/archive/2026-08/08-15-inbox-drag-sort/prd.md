# 收件箱支持 @dnd-kit 拖拽排序

## Goal

收件箱页面（`InboxPage`）复用现有 `SortableTaskRow` 启用 @dnd-kit 拖拽排序，与任务列表、今日任务分组交互一致。

## Background

- `SortableTaskRow`（`TaskRow.tsx:172`）已封装 `useSortable`，供拖拽列表复用。
- 任务列表页与今日任务分组已用 `DndContext` + `SortableContext` + `arrayMove` 实现拖拽排序，模式现成。
- 收件箱已有上移/下移按钮（`moveUp`/`moveDown` → `ipc.taskReorder`），仅缺拖拽手势。
- 后端 `task_reorder` 已按清单分组重写（收件箱为单清单 kind=inbox，天然适用）。

## Requirements

1. 收件箱任务列表启用拖拽排序：`DndContext` + `SortableContext` + `SortableTaskRow`。
2. 拖拽结束后调用 `ipc.taskReorder` 持久化，并刷新 `["tasks"]`（含 `["tasks","inbox"]`）。
3. 拖拽过程中 DragOverlay 跟随 + transform 让位反馈。
4. 保留现有上移/下移按钮（键盘排序通道）。
5. 点击选中、Enter/Space、双击重命名不受影响（PointerSensor distance 8，与现有页面一致）。

## Constraints

- 仅改 `src/features/inbox/InboxPage.tsx`，复用 `SortableTaskRow`，不改 `TaskRow.tsx`。
- 无新依赖（@dnd-kit 已装）。
- 收件箱查询非分页（`taskQuery({ inboxOnly, status: "todo" })` 返回全部），重排基于 `inboxTasks` 全量。

## Acceptance Criteria

- [ ] 收件箱列表按住左键拖动可实时预览目标位置
- [ ] 松开后顺序持久化（刷新后保持），调用 `taskReorder`
- [ ] 拖拽不误触发选中 / 双击重命名
- [ ] 上移/下移按钮仍可用
- [ ] `pnpm typecheck` 与 `pnpm test:unit` 通过

## Notes

- 轻量级任务，PRD-only。
- 参考实现：`TasksPage.tsx`（smart==="none" 分支）与 `TodayPage.tsx`（dueToday 分组）的 DndContext 接线。
