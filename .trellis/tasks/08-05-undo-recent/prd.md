# 撤销/最近操作（undo-recent）

## Goal

为任务的**完成/取消完成/归档/删除**提供短时撤销（RecentAction）：误操作后可一键还原。作为 v1.1「撤销 + 最近操作」的 MVP，先覆盖高频且可逆的任务操作。

## 背景（勘察确认）

- 后端可逆操作：`task_uncomplete`（已存在）；`task_update` 的 `UpdateTaskInput` **不含 status**（client.ts:143-152），无 `task_unarchive` → 撤销归档需新增后端命令。
- 任务删除是软删（deleted_at），但**无恢复命令**；撤销删除需用 `taskCreate` 重建（新 id，保留标题/备注/清单/截止/优先级/标签）。
- 前端有 zustand（`src/stores/ui.ts`）。
- 主要操作入口：TaskDetailPanel（完成/恢复/归档/删除/跳过）、TodayPage 与 TasksPage 的任务勾选（toggle-complete）。

## Requirements

- R1 后端新增 `task_unarchive` 命令：仅当任务 status=archived 时置回 'todo'（沿用 uncomplete_task 模式），注册 lib.rs。
- R2 前端新增 `useRecentActions` zustand store：保存最近动作栈（上限 5），每项 `{ id, label, undo: () => Promise<void> }`。
- R3 新增 `RecentActionToast`（MainShell 底部）：展示最新动作 + 「撤销」按钮，撤销成功刷新 `["tasks"]` 并弹出下一条；5 秒自动消失或点撤销后消失。
- R4 接线动作（mutation onSuccess 里 `pushRecentAction`）：
  - 完成/取消完成（TaskDetailPanel、TodayPage、TasksPage 勾选）；
  - 归档（TaskDetailPanel）→ undo 调 `task_unarchive`；
  - 删除任务（TaskDetailPanel）→ undo 用 `taskCreate` 重建；
  - 跳过周期实例 → undo 用 `taskUncomplete` 恢复（若可逆则加，否则标注边界）。
- R5 撤销后相应 query 失效刷新。

## Acceptance Criteria

- [ ] AC1 `task_unarchive` 后端可用，cargo test 通过。
- [ ] AC2 完成/归档/删除任务后出现「撤销」提示，撤销分别恢复为未完成/未归档/重建任务。
- [ ] AC3 撤销栈上限 5，逐条弹出；自动超时消失。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 中-大复杂度任务：PRD + design。改动为 Rust（task_unarchive）+ 前端（store/toast/接线）。
- MVP 边界：不覆盖延期/移动/清单创建；删除撤销为重建（新 id），原 id 关联（提醒/关联资源）不迁移。
