# Implement: v2.0 复杂信息提取（切片 2）

## 1. 领域层

- [ ] `ExtractApplyInput` / `ExtractApplyResult` + indices 校验（空/去重/越界）单测
- 验证：`cargo test --lib domain::ai_suggestion`

## 2. 服务层（AISuggestionService）

- [ ] `find_pending(feature, source_entity_id)`：幂等查询
- [ ] `request_extract(memory, memories: &MemoryService)`：context 组装（title+body）→ request 管线
- [ ] `apply_extract(input, tasks, links, search)`：门禁→逐项创建（ambiguous 不写日期、notes 来源引用、EntityLink、search.upsert）→ accepted
- [ ] 单测：幂等、重复 apply 拒绝、ambiguous 跳过日期、部分失败保留 pending、link 记录
- 验证：`cargo test --lib application::ai_suggestions`

## 3. IPC + 前端

- [ ] 命令 `ai_extract_request` / `ai_suggestion_apply` + lib.rs 注册
- [ ] client.ts 封装 + `AISuggestionRecord` 前端类型核对
- [ ] `ExtractSuggestionsPanel.tsx`（草稿勾选/ambiguous 标记/excerpt/创建/拒绝/空态）
- [ ] MemoryPage：入口按钮（mode+feature 条件）、面板接入、成功导航
- [ ] 设置页 extract 开关文案更新
- 验证：`pnpm test:unit` + `pnpm build` + `tsc --noEmit`

## 4. 评估与文档

- [ ] `#[ignore]` 在线用例：记忆文本 → extract → apply 全链路（Ollama）
- [ ] docs/ai-assist.md「长文本提取」小节 + PRD AC 勾选
- 验证：`cargo test`（含全部新单测）

## Review gates

1. 步骤 2 后：确认无自由文本直写路径（代码审查）
2. 步骤 3 后：面板文案语气 + 键盘可达性（checkbox 原生控件）
3. 完成后：父任务全局合同 7 条复核 → finish + archive
