# Implement: v2.0 AI 任务拆分（切片 7）

## 1. 领域层

- [ ] `SPLIT_SYSTEM_PROMPT` 开放 + 单测（不建任务/依据子串/宁缺毋滥固化）
- 验证：`cargo test --lib domain::ai_suggestion`

## 2. 服务层

- [ ] `request_split`（冻结/空内容短路/子串校验丢条、全丢作废）
- [ ] `apply_split`（门禁 → checklist_add 逐项 → accepted；部分失败保持 pending）
- [ ] 单测：零任务字段变化、依据过滤、空内容零 provider 调用、重复 apply 拒绝
- 验证：`cargo test --lib application::ai_suggestions`

## 3. IPC + 前端

- [ ] 命令 `ai_split_request` / `ai_split_apply` + lib 注册；client.ts 封装
- [ ] ChecklistSection：AI 拆分按钮 + 草稿区（勾选/依据/都不合适）+ 设置页文案
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 4. 文档与评估

- [ ] ai-assist.md「任务拆分」小节；PRD/implement 勾选
- [ ] `#[ignore]` 在线档（依据子串命中率断言）
- 验证：`cargo test`

## Review gates

1. 步骤 2 后：确认零任务写路径（只写 checklist + ledger）
2. 步骤 3 后：草稿区文案语气
3. 完成后：父任务全局合同复核 → finish + archive
