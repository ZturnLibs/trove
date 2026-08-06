# 触发词展开/插入（snippet-trigger）

## Goal

为「快速文本片段」兑现触发词能力：搜索时触发词可命中记忆；快速记录窗口（capture）中输入触发词时提供「展开」通道，把片段内容带入输入框，实现快速复用。

## 背景（勘察确认）

- 记忆有 `trigger_word` 字段（记忆编辑区可录入，MemoryPage:170-182），QuickWindow 会列出 quickInsert 记忆（QuickWindow.tsx:250-259）。
- 记忆搜索索引只用 `title` + `body`（memories.rs:108 `upsert_conn`），`trigger_word` 不参与检索。
- 目前无「输入触发词自动展开」的通道；现有快速插入只把 body 复制到剪贴板（QuickWindow.tsx:254-256）。
- 前端用 `ipc.memoryQuery({ quickInsertOnly: true })` 获取快速插入记忆。

## Requirements

- R1 后端：记忆搜索索引把 `trigger_word` 并入可搜索文本（`upsert_conn` 时 body 后拼接 trigger_word），使按触发词检索可命中记忆。
- R2 前端：QuickWindow capture 模式加载 `quickInsertOnly` 记忆；当输入（trimmed）恰好等于某记忆的 `triggerWord`（非空）时，在结果列表**顶部**显示「↩ 展开「{title}」」项；选中后把输入替换为该记忆 `body`（body 为空则用 title），供继续提交。
- R3 触发词匹配大小写不敏感（`toLowerCase` 比较）；多个记忆撞触发词时取第一个（或列表全部，以实现为准）。
- R4 展开后不清空 captureType/dueDate 等既有表单状态，仅替换标题输入。

## Acceptance Criteria

- [ ] AC1 搜索按 `trigger_word` 可命中记忆（后端单测验证 upsert 包含 trigger_word）。
- [ ] AC2 输入触发词时顶部出现展开项，选中后输入变为片段内容。
- [ ] AC3 展开不破坏 captureType/日期/优先级等表单状态；无触发词时无额外干扰。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 中复杂度任务：PRD + 简要 design。改动为 memories.rs（搜索索引拼接）+ QuickWindow.tsx。
- 不做系统级「直接粘贴」（辅助功能权限），该能力按用户决策后续处理。
