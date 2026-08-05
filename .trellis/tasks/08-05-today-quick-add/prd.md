# 今日页底部快速输入任务

## Goal

今日页面（`TodayPage`）内容区域底部增加一个常驻的快速输入入口，用户输入内容后按回车即创建任务。输入内容复用现有自然语言解析（`nlParseCapture`）自动提取截止日期/时间/优先级；未指定日期时默认截止日期为今天。创建成功后输入框清空、列表刷新，新任务出现在列表中并可被选中。

用户价值：不用点「新建」再改标题，直接在今日页底部打字回车即可记录待办，降低任务录入成本。

## 背景（代码勘察确认）

- 今日页 `src/features/today/TodayPage.tsx` 使用 `SplitTaskLayout` 的 `list` 区域渲染任务分组（逾期/今日提醒/今日任务/今日已完成），`actions` 已有「新建任务」按钮（`NewTaskButton`）。
- `NewTaskButton` 走 `createMutation`：`ipc.taskCreate({ title: "新任务", dueDate: today })`，创建后选中并聚焦标题，进入内联重命名。
- 自然语言解析能力已在 `QuickWindow.tsx` 实现：`ipc.nlParseCapture(text)` 返回 `{ title, dueDate, dueTime, priority, recurrence, ambiguousFields }`，随后 `taskCreate` / `taskCreateRecurring` 创建任务。本功能可复用该链路。
- IPC 封装与类型位于 `src/ipc/client.ts`（`nlParseCapture`、`taskCreate`、`taskCreateRecurring`、`CreateTaskInput`）。

## Requirements

- R1 今日页内容区域（list 列表）底部常驻一个快速输入框，placeholder 提示可输入自然语言（如「明天下午三点回复客户…」）。
- R2 输入非空内容后按回车创建任务：先用 `nlParseCapture` 解析标题与字段；未指定截止日期时默认使用今日页的 `today`。
- R3 创建成功后清空输入框、刷新任务列表（失效 `["tasks"]` 查询），新任务保持此前「新建任务」的选择与聚焦行为（可选，保持一致性）。
- R4 解析出的截止时间/优先级等随创建一并提交；若解析出重复规则则走 `taskCreateRecurring`。
- R5 创建过程中防重复提交（pending 状态或禁用），失败时在输入框附近给出可读错误提示。
- R6 输入框在空列表（EmptyState）与有列表两种状态下都应可见，位于内容区域底部。

## Acceptance Criteria

- [ ] AC1 今日页内容区域底部显示快速输入框，无列表时（空状态）也可见。
- [ ] AC2 输入「明天下午三点回复客户」回车后，创建截止日期为明天 15:00 的任务，列表出现新任务。
- [ ] AC3 输入「回复客户」回车后，创建截止日期为今天的任务。
- [ ] AC4 创建成功后输入框自动清空，任务列表无需手动刷新即更新。
- [ ] AC5 空内容回车不创建任务；连续快速回车不产生重复任务。
- [ ] AC6 输入「每天 9 点写日报」等含重复规则的内容时按重复任务创建（`taskCreateRecurring`）。

## Notes

- 轻量任务：PRD-only，无设计/实施文档。
- 改动仅涉及前端 `TodayPage.tsx`（及可能复用的输入样式），无 Rust 侧变更。
