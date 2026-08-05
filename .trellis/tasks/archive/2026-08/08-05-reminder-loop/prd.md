# 提醒管理闭环（reminder-loop）

## Goal

补齐提醒的「创建→查看→编辑→删除→完成」闭环：接入后端已就绪但未接前端的 `reminder_update`；为独立提醒提供查看全部/编辑/删除能力；任务提醒支持编辑。用户可修正录错的时间、清理误建提醒，并统一管理未来提醒。

## 背景（勘察确认）

- 后端命令：`reminder_create` / `reminder_update`（commands/mod.rs:269，已注册 lib.rs:526）/ `reminder_delete` / `reminder_list_for_task` / `reminder_complete` / `reminder_snooze`。**无「全部提醒列表」命令**。
- `UpdateReminderInput`（domain/reminder.rs:93）：`{ id, title, notes, fire_at, recurrence, enabled, end_at }`。
- `ReminderService`（reminders.rs）：`update`/`delete`/`list_for_task`/`today_items` 等已实现；**无 `list_all`**。
- client.ts：已封装 `reminderCreate/Delete/ListForTask/Complete/Snooze`，**无 `reminderUpdate`、无 `reminderListAll`**。
- 今日页提醒区（TodayPage.tsx:353-398）：只读展示 + 贪睡/完成，无编辑/删除。
- 任务提醒区（TaskDetailPanel.tsx TaskRemindersSection）：创建 + 删除，无编辑。
- 调度器（scheduler.rs）20s 轮询 `due_occurrences`，修改 `fire_at` 后自然按新时间触发。

## Requirements

- R1 后端新增 `reminder_list_all` 命令 + `ReminderService.list_all()`（返回全部提醒，含 future/禁用，含 nextFireAt），并在 lib.rs 注册。
- R2 client.ts 新增 `reminderUpdate`、`reminderListAll` 封装。
- R3 今日页提醒区：独立提醒可**编辑**（标题/计划时间/备注/启用）与**删除**。
- R4 今日页提供「全部提醒」视图/入口：列出所有提醒（含未来与已禁用），可编辑/删除/切换启用。
- R5 任务提醒区（TaskDetailPanel）：支持编辑（时间/重复/启用）与删除（保留既有删除）。
- R6 编辑/删除/禁用后失效 `["tasks"]`、`["reminders"]` 相关查询并刷新；修改 `fire_at` 后新时间生效。

## Acceptance Criteria

- [ ] AC1 后端 `reminder_list_all` 可用，返回全部提醒（含未来/禁用），cargo test 通过。
- [ ] AC2 client.ts 提供 `reminderUpdate`/`reminderListAll`，前端可调用。
- [ ] AC3 今日页独立提醒可编辑标题/时间/备注，可删除；删除后列表即时更新。
- [ ] AC4 今日页「全部提醒」入口可查看所有提醒，编辑/删除/禁用即时生效。
- [ ] AC5 任务提醒可编辑时间/重复/启用。
- [ ] AC6 `pnpm typecheck`、`pnpm build`、`cargo check` 通过。

## Notes

- 轻量-中复杂度任务：PRD + 简要 design。改动含 Rust（新增 list_all 命令）与前端（今日页/任务详情）。
- 不与重复触发等既有行为冲突；`reminder_update` 复用现有服务实现。
