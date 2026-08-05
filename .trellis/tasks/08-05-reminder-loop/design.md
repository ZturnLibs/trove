# 技术设计：提醒管理闭环

## 1. 后端：全部提醒列表

新增 `ReminderService.list_all()`（reminders.rs）：

```rust
pub fn list_all(&self) -> Result<Vec<Reminder>, DomainError> {
    // SELECT * FROM reminders ORDER BY next_fire_at（含禁用）；next_fire_at 由已存字段提供
}
```

- `reminders` 表已含 `enabled`/`next_fire_at`/`end_at` 等字段（Reminder struct 即映射全表），无需迁移。
- 新命令 `reminder_list_all`（commands/mod.rs）+ `lib.rs` 注册：

```rust
#[tauri::command]
pub fn reminder_list_all(state: State<'_, AppState>) -> Result<Vec<Reminder>, AppError> {
    state.reminders.list_all().map_err(Into::into)
}
```

## 2. 前端 IPC

client.ts 新增（对齐 camelCase 序列化）：

```ts
reminderUpdate: (input: UpdateReminderInput) => invoke<Reminder>("reminder_update", { input }),
reminderListAll: () => invoke<Reminder[]>("reminder_list_all"),
```

新增类型 `UpdateReminderInput`（对应 Rust UpdateReminderInput：id/title/notes/fireAt/recurrence/enabled/endAt）。

## 3. 今日页提醒区改造（TodayPage）

- 独立提醒详情（右侧 selectedReminder）：
  - 新增「编辑」状态：标题 Input、计划时间 `<input type="datetime-local">`（值取 occurrence.scheduledAt 前 16 位）、备注 textarea、启用开关；
  - 「保存」→ `reminderUpdate({ id, title, notes, fireAt: 规范化 "YYYY-MM-DDTHH:MM:SS", enabled })`，成功失效 `["tasks"]` 并刷新；
  - 新增「删除」ConfirmButton → `reminderDelete(reminder.id)`，成功清空选中并刷新。
- 顶部 actions 新增「全部提醒」开关：
  - 开启后 list 区展示 `reminderListAll` 查询结果（标题/下次时间/启用态/关联任务），每行提供编辑（复用同一编辑弹层/内联）与删除；默认仍显示今日提醒。
  - 用 `useQuery({ queryKey: ["reminders", "all"] })`。

## 4. 任务提醒编辑（TaskDetailPanel TaskRemindersSection）

- 列表每行增加「编辑」：展开一个表单（时间 datetime-local + 重复 checkbox + 启用 checkbox），保存走 `reminderUpdate`；保留既有删除。
- 沿用现有 `["reminders","task",taskId]` query key，成功后失效刷新。

## 5. 交互/一致性

- 时间统一规范化为 `"YYYY-MM-DDTHH:MM:SS"`（沿用既有 create 的 `normalized.replace(" ", "T")` 模式）。
- 时区沿用系统时区（`Intl.DateTimeFormat().resolvedOptions().timeZone`），与既有 UI 路径一致（不动 NL 解析）。
- 删除/禁用独立提醒后，若它在今日提醒区被选中，需清空选中态。

## 6. 回归注意

- `reminder_update` 后端已实现并测试覆盖 update 逻辑（reminders.rs tests），只新增 list_all 与命令层，行为不回归。
- 调度器 20s 轮询因 next_fire_at 变化自动适配，无需改动。
