# 模板能力补齐（template-polish）

## Goal

补齐模板的「创建（通用）→ 应用前预览确认 → 应用」链路：把设置页模板区从 2 个硬编码示例改为三类型通用创建表单；应用模板前调用已有 `template_preview` 展示结果并二次确认，兑现「模板执行前展示结果」承诺。

## 背景（勘察确认）

- 命令：`template_list` / `template_create` / `template_delete` / `template_preview`（commands/mod.rs:475-498）/ `template_apply`（:501）。
- `templatePreview(id)` 已在 client.ts:477 封装，但全库无调用；`TemplatePreview` = `{ kind, title, body, dueDate, dueTime, fireAt, priority, recurrence, tagNames }`。
- `templateCreate` 已支持三类型（`CreateTemplateInput { kind, name, payload }`），payload 语义（templates.rs `preview`）：
  - task：`title`、`notes`、`priority`、`tagNames`、`relativeDueDays`、`dueTime`；
  - reminder：`title`、`relativeFireHours`、`fireAt`（绝对）、`recurrence`；
  - memory：`title`、`body`、`tagNames`。
- 现状：设置页模板区仅「添加示例：周报/报销」两个硬编码任务模板按钮（SettingsPage.tsx:319-364）；列表「应用」直接 `templateApply` 无预览（:376-384）；QuickWindow 命令面板一键应用（QuickWindow.tsx:216-226）。
- 模板列表 queryKey `["templates"]`。

## Requirements

- R1 设置页模板创建区改造为**通用创建表单**：名称 + 类型（task/reminder/memory）+ 类型相关字段（task：标题/相对截止天数/优先级；reminder：标题/相对小时/每天重复；memory：标题/正文），提交走 `templateCreate`。
- R2 设置页模板列表「应用」前调用 `templatePreview`，弹出预览面板（类型/标题/截止或触发时间/优先级/标签/周期），确认后才 `templateApply`；取消不应用。
- R3 应用/创建成功后失效 `["templates"]` 与 `["tasks"]`/`["reminders"]`/`["memories"]` 相关查询，并给出提示（沿用 `setMessage`）。
- R4 保留既有「删除」；QuickWindow 命令面板一键应用保留为快路径（不强制预览，作为已知边界）。

## Acceptance Criteria

- [ ] AC1 设置页可通用创建 task/reminder/memory 三类模板，列表即时出现。
- [ ] AC2 「应用」先展示预览（含解析后的截止/触发时间、优先级、标签），确认后创建实体；取消不创建。
- [ ] AC3 应用/创建后列表与业务查询刷新，提示可见。
- [ ] AC4 `pnpm typecheck`、`pnpm build` 通过。

## Notes

- 中复杂度任务：PRD + 简要 design。纯前端改动（SettingsPage.tsx 为主），无 Rust 改动。
- 不删除「添加示例」按钮（保留快捷示例），新增通用创建区与其并存；或按实现便捷度用通用表单替代示例按钮（以 AC1 为准）。
