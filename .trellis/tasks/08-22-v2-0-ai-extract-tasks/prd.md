# v2.0 复杂信息提取（切片 2）

> 父任务：`08-19-v2-0-ai-assist-roadmap`（全局合同见父 PRD）。依赖切片 1（已交付：provider 边界/台账/设置页）。

## Goal

把「会议记录/邮件等长文本 → 候选任务」跑通端到端：记忆详情一键提取 → 草稿列表（原文引用、模糊日期待确认）→ 勾选确认创建。这是 §9.3「复杂信息提取」的第一个用户可见 AI 功能。

## 入口决策

首切片只做 **MemoryPage 记忆详情**入口（记忆是长文本的天然存放地，与既有「转任务」按钮并列）；QuickWindow / 剪贴板行动气泡入口留给后续切片复用同一条管线，不阻塞本切片验收。

## Requirements

1. `ai_extract_request(memoryId)`：组装该记忆 title+body 为 context → 切片 1 管线（开关/sanitize/provider/校验）→ pending 建议。同一记忆已有 pending extract 时不重复调用 provider（幂等返回已有记录）。
2. `ai_suggestion_apply(id, selectedIndices)`：
   - 门禁：建议必须处于 pending；indices 合法（去重、越界拒绝）；
   - 逐项 `taskCreate`（收件箱默认清单），`ambiguous=true` 或日期非法的项**不写日期**（§9.3 不猜测写入），notes 追加来源引用（「来源：记忆《标题》」）；
   - 每个创建的任务写 `EntityLink(memory → task, "ai_extract")`（可审计关联）；
   - 成功后建议置 accepted；返回创建的任务列表。
   - 单项创建失败整体报错回滚语义：已创建的任务保留（业务数据不丢），建议保持 pending 并报错，用户可重试剩余项（indices 去重后幂等）。
3. UI（记忆详情区）：
   - 「提取任务」按钮：仅当 `ai.mode != off && ai.features.extract` 显示；点击 loading → 结果面板；
   - 草稿面板：每项 checkbox（默认全不勾）、标题、日期/时间（ambiguous 显示「日期待确认，创建时不填」）、原文摘录（等宽小字）、「创建选中任务」按钮；
   - 全部拒绝 → 「都不合适」按钮置 rejected；
   - provider 不可用/无结果 → 空状态文案（不 guilt-trip）；
   - 创建成功 → 列出已创建任务标题，可点击跳转（复用任务导航）。
4. 设置页：extract 开关文案从「随后续版本开放」改为可用说明。
5. 评估：在线档 `#[ignore]` Ollama 回归用例覆盖记忆→任务提取全链路。

## Acceptance Criteria

- [ ] 记忆详情 3 步完成提取→勾选→创建（提取 1 次 + 创建 1 次点击 + 可选勾选）
- [ ] 每条草稿显示原文摘录；点击建议来源可对应到记忆正文（excerpt 即引用）
- [ ] ambiguous/非法日期不写入任务（单测断言 due_date 为空）
- [ ] 只创建勾选项；重复 apply 同一建议被拒绝（幂等门禁单测）
- [ ] AI 关闭或功能开关关闭时入口不出现；provider 失败仅显示降级文案
- [ ] 创建的任务带来源 notes + EntityLink；撤销删除任务不影响建议状态（审计留存）
- [ ] cargo test 全绿（新增：apply 事务/越界/重复/ambiguous/链路）+ pnpm test/build 通过
- [ ] docs/ai-assist.md 更新「长文本提取」使用说明；设置页文案更新

## 明确不做（本切片）

- QuickWindow / 剪贴板入口（后续切片）
- 草稿项的编辑/改日期（勾选创建后用既有任务编辑完成，避免双编辑面）
- 提取提醒/周期（只提取任务标题+日期）
- 语义检索 / 相关内容 / 摘要（Wave B/C）
