# Implement: v2.0 周期回顾摘要（切片 3）

## 1. 领域层

- [x] `AIFeature::Summary.prompt_template()` 开放 + prompt 文案单测（禁评价约束在模板常量）
- 验证：`cargo test --lib domain::ai_suggestion`

## 2. 服务层

- [x] `dismiss_pending(feature, source)` 私有方法
- [x] `request_weekly_summary(...)`：snapshot → 标题 context（剪贴板组 sanitize）→ request
- [x] 单测：re-request 置 dismissed 旧条、context 仅含标题、schema items 空
- 验证：`cargo test --lib application::ai_suggestions`

## 3. IPC + 前端

- [x] 命令 `ai_weekly_summary_request` + lib 注册；`weekly_review_complete` 追加清理 pending
- [x] client.ts 封装；WeeklyReviewPage `WeeklySummaryCard`（三态 + 徽标 + 降级文案 + 恢复 pending）
- [x] 设置页 summary 开关文案「已开放」
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 4. 文档与评估

- [x] ai-assist.md「每周回顾摘要」小节；PRD/implement 勾选
- [x] `#[ignore]` 在线档：真实模型生成摘要（items 空 + summary 非空）
- 验证：`cargo test`

## Review gates

1. 步骤 2 后：context 组装代码审查（无正文/无敏感）
2. 步骤 3 后：摘要卡片文案语气审查
3. 完成后：父任务全局合同复核 → finish + archive
