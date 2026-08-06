# smoke_notes 接入 UI（smoke-notes-ui）

## Goal

把已就绪但无 UI 的 `smoke_note_*` 快速记录（随手记）后端接入前端：快速记录窗口新增「随手记」类型，记忆页提供「记忆/随手记」视图以浏览与删除，消除死代码。

## 背景（勘察确认）

- 后端 `smoke_note_create/list/delete` 命令已注册（commands/mod.rs:160-171），`SmokeNoteService` 已注入 app_state，`SmokeNote { id, body, createdAt, updatedAt, revision }`（client.ts:82-88）。
- client.ts 已有 `smokeNoteCreate(body)` / `smokeNoteList()` / `smokeNoteDelete(id)`（:452-454），全库无调用。
- QuickWindow capture `CaptureType = "task" | "reminder" | "memory"`（QuickWindow.tsx:21），模式 tab 在 :437-441。
- 记忆页 MemoryPage 使用 `SplitTaskLayout`，有 actions 区与列表区。
- data_port.rs 导出 smoke_notes 表（备份/导入涉及）。

## Requirements

- R1 QuickWindow capture 新增第 4 类型「随手记」（`CaptureType` 增加 `"note"`）：输入文本后回车/提交调用 `ipc.smokeNoteCreate(body)`，成功后清空并保持窗口或关闭（沿用现有提交行为）。
- R2 记忆页（MemoryPage）增加「记忆/随手记」视图切换：随手记视图列出 `ipc.smokeNoteList()`（按时间倒序），每条可「删除」（ConfirmButton）；提供顶部小输入框快速新建。
- R3 创建/删除后失效 `["smoke-notes"]` 查询并刷新。
- R4 随手记视图与既有记忆视图互不干扰（切换时保留各自选中/筛选状态即可）。

## Acceptance Criteria

- [ ] AC1 快速记录窗口可选「随手记」并成功创建，创建后列表/清空行为正常。
- [ ] AC2 记忆页可切换到「随手记」视图，看到全部随手记并可删除。
- [ ] AC3 创建/删除即时刷新；与记忆视图切换无冲突。
- [ ] AC4 `pnpm typecheck`、`pnpm build` 通过（无 Rust 改动）。

## Notes

- 中复杂度任务：PRD + 简要 design。纯前端改动（QuickWindow.tsx、MemoryPage.tsx）。
- 决策已确认：接入 UI（不清理后端）。
