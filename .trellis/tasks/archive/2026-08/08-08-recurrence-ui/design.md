# Design: 周期提醒 UI 补全

## Component API

```typescript
// RecurrencePicker.tsx
export type RecurrencePickerProps = {
  value: RecurrenceRule | null;
  onChange: (rule: RecurrenceRule | null) => void;
  disabled?: boolean;
  className?: string;
};
```

- `value === null` → 「不重复」
- `onChange(null)` → 清除周期

## UI structure

```text
[ ] 重复
  └─ (when checked)
       频率: [每天 ▼]
       (weekly)  星期: [一][二]...[日]
       (monthly) 每月第 [15] 日
       (everyNDays) 每 [3] 天
       (everyNWeeks) 每 [2] 周
       摘要: 「每两周 · 周一、周三」
```

样式遵循现有 `TaskDetailPanel` 内 `text-[11px] text-muted` + `Input` / native `select` 模式，不引入新依赖。

## Shared utilities

提取到 `src/lib/recurrence.ts`：

- `recurrenceLabel(rule: RecurrenceRule): string` — 从 `SettingsPage.tsx:39-48` 迁出
- `defaultRecurrence(timezone?: string): RecurrenceRule` — `{ version:1, frequency:'daily', interval:1, timezone }`
- `recurrenceFromNl(parsed: RecurrenceRule | null): RecurrenceRule | null` — 直通，供 QuickWindow 回填

## Integration map

| File | Change |
| --- | --- |
| `RecurrencePicker.tsx` | 新组件 |
| `recurrence.ts` |  label + helpers |
| `TaskDetailPanel.tsx` | 替换 `recurring` boolean ×2 |
| `QuickWindow.tsx` | 移除 `daily` state；`parsed.recurrence` → picker |
| `TodayPage.tsx` | `ReminderEditForm` 增加 picker；`save()` 提交 `recurrence` from picker |
| `SettingsPage.tsx` | 模板提醒创建用 picker；删除本地 `recurrenceLabel` |

## NL backfill flow (QuickWindow)

```text
onTitleBlur / parse
  → ipc.nlParseCapture
  → if parsed.recurrence: setRecurrence(parsed.recurrence)
  → user adjusts RecurrencePicker
  → submit: use picker value (not daily boolean)
```

## Backend touchpoints

- 无 Rust 改动预期（`reminder_create` / `reminder_update` / `task_create_recurring` 已接受 `RecurrenceRule`）。
- 实现前 `rg recurrence` 确认 `UpdateTaskInput` 仍无 recurrence——任务编辑不做。

## Testing

- 单元：`recurrenceLabel` 各 frequency 输出中文摘要。
- 手动：创建 weekly 提醒 → 重启 app → 编辑表单显示 weekly。
