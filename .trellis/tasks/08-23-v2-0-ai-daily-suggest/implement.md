# Implement: v2.0 每日工作建议（切片 5）

## 1. 领域层

- [ ] `SUGGEST_SYSTEM_PROMPT` 开放 + 单测（特征引用/完全一致/宁缺毋滥固化）
- 验证：`cargo test --lib domain::ai_suggestion`

## 2. 服务层

- [ ] `reject_related_item` 泛化为 `reject_suggestion_item(feature 校验 + 配对 status 参数)`
- [ ] `request_daily_suggest`：跨天收口 + 重生成收口 + 候选（today_tasks 差集 + 特征行 ≤15）+ 回配
- [ ] `daily_skipped_ids` 过滤；`remove_daily_suggest_item(id, index, accepted)`
- [ ] 单测：不自动入重点、focus/waiting 排除、跳过降频、跨天收口、编造丢弃
- 验证：`cargo test --lib application::ai_suggestions`

## 3. IPC + 前端

- [ ] 命令 `ai_daily_suggest_request` / `ai_daily_suggest_skip` + lib 注册
- [ ] client.ts 封装；`DailySuggestionsCard.tsx`（加入重点走 dailyFocusAdd + 撤销栈）
- [ ] TodayPage 接入（智能排序区上方）；设置页 suggest 开关「已开放」
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 4. 文档与评估

- [ ] ai-assist.md「今日工作建议」小节；PRD/implement 勾选
- [ ] `#[ignore]` 在线档：真实模型（回配 + 理由基于特征）
- 验证：`cargo test`

## Review gates

1. 步骤 2 后：确认零写路径（不动 tasks/focus 表）
2. 步骤 3 后：卡片文案语气（建议措辞，不指令）
3. 完成后：父任务全局合同复核 → finish + archive
