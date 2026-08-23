# v2.0 周期回顾摘要（切片 3）

> 父任务：`08-19-v2-0-ai-assist-roadmap`。依赖切片 1（已交付）；复用 `SuggestionContent.summary` 输出通道（为此设计）。
> §9.3 对应：周期回顾摘要——「所有数字由确定性查询产生，模型只负责组织文字」。

## Goal

每周回顾页新增「AI 摘要」：把 `WeeklyReviewSnapshot` 的确定性统计数字交给模型组织成一段温和的中文小结（不做评价、不打分、不 guilt-trip），作为建议展示；用户可重新生成或忽略。数字与结构完全来自 `snapshot()`，模型输出仅文字。

## Requirements

1. Prompt（`AIFeature::Summary.prompt_template()` 开放）：
   - 输入：snapshot 的计数 + 各分组前若干任务**标题**（不带正文，最小上下文）；
   - 输出：`{"summary": string, "items": []}`（走既有 schema；items 为空、summary 非空合法——切片 1 校验已支持）；
   - 文案约束写入 prompt：只陈述事实、不评价、不催促、不用「落后/失败」类词；≤200 字。
2. 服务 `request_weekly_summary(snapshot, tasks, reminders, clipboard)`：
   - sanitize：标题列表过滤敏感来源（剪贴板项 source_app 判断；记忆不参与）；
   - 幂等：以 `source_entity_id = "weekly"` 的 pending summary 建议**每次重新生成**（摘要时效性强，不沿用 pending 幂等；生成前把上一条 pending 置 dismissed）；
   - 落库 `feature_type=summary`。
3. UI（WeeklyReviewPage 顶部卡片）：
   - 入口：AI 可用（mode≠off && features.summary）时显示「生成 AI 摘要」按钮；
   - 展示：摘要段落 + 「基于本周确定性统计生成」说明 + 每类数字徽标（可对照）+ 重新生成 / 忽略按钮；
   - 忽略 → `aiSuggestionDecide(dismiss)`；摘要不写回 ReviewSession（会话 summary 仍是确定性 JSON）。
4. 完成回顾时若摘要仍 pending → 自动 dismiss（不留悬置建议）。

## Acceptance Criteria

- [ ] 摘要中的所有数字均来自 snapshot 确定性查询；模型输出只含文字（schema items 必空，单测断言）
- [ ] prompt 不含任务正文/剪贴板正文，仅标题（代码审查 + 单测构造 context 断言）
- [ ] 重新生成会把上一条 pending 置 dismissed（单测）
- [ ] AI 关闭/开关关闭时页面无 AI 痕迹；provider 失败显示降级文案不阻塞回顾流程
- [ ] 完成回顾自动清理 pending 摘要（单测）
- [ ] cargo test / pnpm test / build 全绿；docs/ai-assist.md 更新

## 明确不做（本切片）

- 摘要写入 ReviewSession 持久化（会话记录保持确定性）
- 每日收尾摘要（focus-daily-wrap 不动）
- 任务名点击跳转（摘要为纯文字段落；跳转属「相关内容」/列表卡既有能力）
- 建议历史的摘要列表 UI（用设置页既有历史区）
