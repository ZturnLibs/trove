# Implement: v2.0 相关内容建议（切片 4）

## 1. 领域层

- [x] `RELATED_SYSTEM_PROMPT` 开放 + 单测（标题一致约束/防编造条款固化）
- 验证：`cargo test --lib domain::ai_suggestion`

## 2. 服务层

- [x] `rejected_pair_ids`、`request_related`（FTS 候选 + 三重过滤 + 标题回配）
- [x] `confirm_related`（幂等 link → accepted）、`reject_related_item`（dismissed 配对 + 自动收口）
- [x] 单测：敏感/已链接/已拒绝过滤、编造标题丢弃、仅 request 无 link、confirm 幂等、reject 后不再出现
- 验证：`cargo test --lib application::ai_suggestions`

## 3. IPC + 前端

- [x] 命令 3 个 + lib 注册；client.ts 封装
- [x] `RelatedSuggestionsSection.tsx`；TaskDetailPanel 接入（附件区上方，条件渲染）
- [x] 设置页 related 开关文案「已开放」
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 4. 文档与评估

- [x] ai-assist.md「相关内容建议」小节；PRD/implement 勾选
- [x] `#[ignore]` 在线档：真实模型回配率（输出标题全部命中候选）
- 验证：`cargo test`

## Review gates

1. 步骤 2 后：回配与过滤代码审查（无自动 link 路径）
2. 步骤 3 后：区文案语气（推荐措辞不越权，如「可能相关」）
3. 完成后：父任务全局合同复核 → finish + archive
