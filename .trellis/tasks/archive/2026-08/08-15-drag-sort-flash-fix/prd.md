# 修复拖拽排序放手后闪回原位

## Goal

修复任务拖拽排序的一个体验缺陷：放手后任务先闪回原位置，再跳到新位置。通过 `handleDragEnd` 的乐观更新（`setQueryData`）让任务立即落在新位置。

## Background

- 三个页面（`TasksPage`、`TodayPage`、`InboxPage`）的 `handleDragEnd` 当前逻辑：计算 `arrayMove` 后的 `orderedIds` → `reorderMutation.mutate(orderedIds)` → `onSuccess` 里 `invalidateQueries(["tasks"])` 重新 fetch。
- 缺点：`mutate` 到 `invalidate` 完成之间有网络往返，期间 React Query cache 仍为旧顺序，@dnd-kit 的 `onDragEnd` 后组件回到旧数据 → 已渲染的列表「闪回」原位置，随后才跳到新位置。
- 修复：`mutate` 前先用 `queryClient.setQueryData` 对该页 query key 做乐观重排，立即反映新顺序。

## Requirements

1. 三个页面的 `handleDragEnd` 在 `reorderMutation.mutate(orderedIds)` 前，用 `setQueryData` 乐观更新对应 query key 的列表顺序。
   - `InboxPage`：key `["tasks","inbox"]`，重排 `data.items`。
   - `TodayPage`：key `["tasks","today"]`，重排 `data.dueToday`。
   - `TasksPage`：key `["tasks","list",...]`，重排 usePagedQuery 的 items 数组。
2. 乐观更新基于 `orderedIds`（含 onDragEnd 中已计算的 arrayMove 结果）重排当前列表，而非依赖索引。
3. 保留现有 `invalidateQueries`（onSuccess 兜底，保证与后端一致）。
4. 不改后端、不改 `TaskRow.tsx`、不引入依赖。

## Constraints

- 仅改 `TasksPage.tsx` / `TodayPage.tsx` / `InboxPage.tsx` 三个文件的 `handleDragEnd`。
- 重排逻辑安全处理 cache 为 undefined 的情况。
- 对 usePagedQuery（TasksPage）cache 为 items 数组（`T[]`），需按 id 顺序稳定重排；分页加载更多（extraItems）与乐观更新的一致性由 onSuccess invalidate 兜底。

## Acceptance Criteria

- [ ] 三个页面拖拽放手后任务立即出现在新位置，不再闪回原位
- [ ] 顺序持久化仍正确（invalidate 后与后端一致）
- [ ] 拖拽取消（handleDragCancel）不受影响
- [ ] `pnpm typecheck` 与 `pnpm test:unit` 通过

## Notes

- 轻量级修复，PRD-only。
- 复现路径：任务列表 / 今日任务 / 收件箱任一，按住任务行拖动后放手。
