# 任务标签筛选/浏览（task-tag-filter）

## Goal

在任务页（TasksPage）增加标签筛选与浏览入口，激活后端已就绪的 `TaskQuery.tagId`，让按标签找任务不再需要开全局面板。

## 背景（勘察确认）

- `TaskQuery.tagId` 已在 client.ts:159 与 Rust `query_tasks`（tasks.rs:553-556）就绪，TasksPage 未传该参数。
- `ipc.taskListTags()`（client.ts:462）已封装 `task_list_tags`（返回全局 tags 表），TasksPage 未调用。
- TasksPage（TasksPage.tsx:133-183）actions 区已有：智能列表、清单、状态、优先级四个下拉；查询 key `["tasks","list",listId,status,priority,smart]`（:49）。
- 标签在任务详情面板（TaskDetailPanel）用逗号分隔输入维护。

## Requirements

- R1 TasksPage 增加标签筛选：`tagId` state + 标签下拉（数据 `ipc.taskListTags()`，含「全部标签」），仅 `smart === "none"` 时显示（与清单/状态/优先级同级）。
- R2 `taskQuery` 透传 `tagId`，queryKey 加入 `tagId`。
- R3 空状态：有标签筛选且无结果时文案与「清除筛选」动作把标签一并清空。
- R4 选中任务后标签下拉仍可用；切换标签后选中项若不在结果集则清空选中（沿用现有 queryKey 变化即重查的行为即可）。

## Acceptance Criteria

- [ ] AC1 任务页出现标签下拉（含「全部标签」），选择后列表按标签过滤。
- [ ] AC2 标签筛选与清单/状态/优先级叠加生效；与智能列表互斥（智能列表时隐藏）。
- [ ] AC3 无匹配结果时空状态出现，并可一键清除全部筛选（含标签）。
- [ ] AC4 `pnpm typecheck`、`pnpm build` 通过。

## Notes

- 轻量任务：PRD-only，纯前端（TasksPage.tsx），无 Rust 改动。
- 标签列表查询用 `["task-tags"]` query key（`useDomainInvalidation` 已覆盖失效）。
