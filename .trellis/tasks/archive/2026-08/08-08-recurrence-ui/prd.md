# 周期提醒 UI 补全（v1.2.1）

## Goal

将提醒/任务创建与编辑处的「每天重复」复选框升级为完整 `RecurrenceRule` 选择器，使用户可在 UI 中设置每天 / 工作日 / 每周 / 每月 / 每隔 N 天（周），并与自然语言解析结果互通。满足 `docs/next-iteration-roadmap.md` §4.2。

**用户价值**：无需依赖 NL 输入即可精确设置周期；NL 解析出的 weekly/monthly 等规则可在表单中可见、可改。

## Background

- 后端 `RecurrenceRule` 类型已完整（`client.ts:168-184`；Rust domain 同步），调度器支持多 frequency。
- 前端所有周期入口仍硬编码 `frequency: "daily"`：
  - `TaskDetailPanel.tsx:352-358`（任务提醒创建）
  - `TaskDetailPanel.tsx:487-493`（任务提醒编辑）
  - `QuickWindow.tsx:341-377`（快速记录，`daily` state 布尔化）
  - `SettingsPage.tsx:272-276`（模板创建提醒）
- `SettingsPage.tsx:39-48` 已有 `recurrenceLabel()` 展示文案，可复用。
- `TodayPage` 独立提醒编辑（`ReminderEditForm`）保存时透传 `reminder.recurrence`，**无 UI 修改周期**（`TodayPage.tsx:143`）。
- 路线图 §4.2 还要求周期**任务**详情可改 `recurrence`；当前 `UpdateTaskInput`（`client.ts:143-152`）无 recurrence 字段——**若后端未暴露 task recurrence update，本期仅覆盖提醒路径**，任务 recurrence 编辑列为 follow-up。

## Requirements

### R1 共享周期选择器组件

- 新增 `RecurrencePicker`（建议路径 `src/design-system/patterns/RecurrencePicker.tsx`）。
- 支持 frequency：`daily` | `weekdays` | `weekly` | `monthly` | `everyNDays` | `everyNWeeks`（与 `RecurrenceFrequency` 一致）。
- 交互：
  - 「不重复」/ 「重复」开关；
  - 重复时：频率下拉 + 条件字段（weekly → 星期几多选；monthly → 日期 1-31；everyNDays/everyNWeeks → interval 数字）。
  - `timezone` 默认 `Intl.DateTimeFormat().resolvedOptions().timeZone`；`version: 1`。
- 展示：选中规则的人类可读摘要（复用/抽取 `recurrenceLabel`）。

### R2 接入创建/编辑入口

| 位置 | 场景 |
| --- | --- |
| `TaskDetailPanel` `TaskRemindersSection` | 任务提醒创建 + `TaskReminderEditRow` 编辑 |
| `QuickWindow` | 任务/提醒快速创建；NL 解析后回填 picker |
| `TodayPage` `ReminderEditForm` | 独立提醒编辑（当前只读 recurrence） |
| `SettingsPage` | 提醒模板创建处的周期选项 |

### R3 NL 互通

- `QuickWindow`：`nlParseCapture` 返回 `parsed.recurrence` 时，写入 picker state（不仅 daily 布尔）；用户仍可在提交前修改。
- `TodayPage` 快速添加：`parsed.recurrence` 走 `taskCreateRecurring` 前，若将来有 picker 则回填（本期至少 QuickWindow + 详情面板）。

### R4 行为约束

- 关闭重复 → `recurrence: null`。
- 编辑已有提醒：picker 初始值来自 `reminder.recurrence`；修改后 `reminderUpdate` 提交完整 rule。
- 不改变调度、贪睡、通知逻辑。

## Out of Scope

- 周期**任务**的 recurrence 编辑（需后端 `UpdateTaskInput.recurrence`，单独 follow-up）。
- `endAt` 重复结束日期 UI（后端支持但使用频率低；可后续加）。
- 复杂 RRULE 表达式编辑器。

## Acceptance Criteria

- [ ] AC1 UI 可设置 weekly / monthly / weekdays / everyNDays，保存后 `reminder.recurrence` 字段正确持久化。
- [ ] AC2 编辑已有 weekly 提醒时 picker 显示正确频率与参数；改为 monthly 后下一实例按新规则（手动或单测验证调度输入）。
- [ ] AC3 QuickWindow 输入「每周一提醒开会」类 NL 后，picker 显示 weekly（非仅 daily 勾选）。
- [ ] AC4 `TaskDetailPanel` / `TodayPage` / `QuickWindow` / `SettingsPage` 四处入口均使用 `RecurrencePicker`，无残留「每天重复」硬编码 checkbox。
- [ ] AC5 `pnpm typecheck`、`pnpm build` 通过；可选 vitest 覆盖 `recurrenceLabel` / picker 纯函数。

## Key Decisions

| 决策 | 选择 | 理由 |
| --- | --- | --- |
| 组件形态 | 共享 `RecurrencePicker` | 4+ 入口一致；减少重复 |
| everyN 上限 | interval 1-365 | 与后端校验对齐（实现时确认） |
| 任务 recurrence 编辑 | 本期不做 | `UpdateTaskInput` 无字段；范围可控 |

## Risks & Deferred

- **后端 validation 边界**：需确认 Rust 对 `weekdays`/`monthday` 校验错误信息能展示到 UI。
- **TodayPage 新建提醒**：当前一键创建无 picker；可在编辑表单打开后即可改周期（AC2 覆盖编辑路径）。
