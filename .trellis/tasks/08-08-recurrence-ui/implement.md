# Implement: 周期提醒 UI 补全

## Checklist

### Phase A — Shared utilities & component

- [ ] A1 新增 `src/lib/recurrence.ts`（`recurrenceLabel`、`defaultRecurrence`）
- [ ] A2 新增 `RecurrencePicker.tsx`
- [ ] A3 可选 `recurrence.test.ts` 覆盖 label

### Phase B — Wire up entry points

- [ ] B1 `TaskDetailPanel`：`TaskRemindersSection` 创建 + `TaskReminderEditRow` 编辑
- [ ] B2 `QuickWindow`：替换 `daily`；NL 回填
- [ ] B3 `TodayPage`：`ReminderEditForm` 增加 picker state
- [ ] B4 `SettingsPage`：模板提醒区 + 删除重复 `recurrenceLabel`

### Phase C — Verify

- [ ] C1 `pnpm typecheck && pnpm build`
- [ ] C2 手动：weekly 创建 → 列表显示「周期」+ 编辑回填
- [ ] C3 手动：QuickWindow NL「每周五」→ picker 为 weekly

## Validation commands

```bash
pnpm typecheck
pnpm test:unit
pnpm build
```

## Risky files

- `QuickWindow.tsx` — NL + 提交路径分支多
- `TaskDetailPanel.tsx` — 提醒 create/update 两套表单

## Suggested order

**先 recurrence-ui（纯前端、无 IPC 破坏）→ 再 data-pagination**，除非用户希望并行；recurrence 可独立合并。

## Rollback

删除 `RecurrencePicker` + 恢复 boolean checkbox 即可；无数据迁移。
