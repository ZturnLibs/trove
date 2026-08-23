# v2.0 相关内容建议（切片 4）

> 父任务：`08-19-v2-0-ai-assist-roadmap`。依赖切片 1（管线）。
> §9.3 对应：相关内容建议——「推荐只作为独立区域展示，不自动创建永久关联；用户确认后才写入 EntityLink；支持对单条建议反馈不相关，减少类似推荐」。

## Goal

任务详情面板新增「相关内容建议（AI）」区：打开任务时可请求推荐，AI 从**本地 FTS 检索出的候选**（记忆/剪贴板）中挑选真正相关的前几条并给出理由；用户逐条「关联」或「不相关」，关联才写 `related` EntityLink。

## Requirements

1. 候选生成（确定性，无模型）：`search.query(任务标题+说明, types=[Memory, Clipboard], limit=12)`；过滤：敏感记忆、已关联实体（task 的双向链接）、**历史已拒绝配对**（ledger 中该 task 的 rejected/dismissed related 记录 sources）。
2. 模型职责（`AIFeature::Related.prompt_template()` 开放）：从候选列表（编号+标题+20 字摘要）挑选相关项 + 一句话理由；**输出必须原样复制候选标题**，服务端按标题精确回配候选 id，配不上的条目直接丢弃（防编造）。
3. 服务：
   - `request_related(task_id, tasks, search, memories)`：dismiss 旧 pending → 候选 → 空则 None → prompt → 回配 → pending 记录（sources 携带 target 实体类型/id）；
   - `confirm_related(suggestion_id, indices, tasks, links)`：选中项逐个 `link("task", …, "related")`（幂等，已存在则跳过）→ accepted，返回链接列表；
   - `reject_related_item(suggestion_id, index)`：单条不相关 → 落一条 dismissed 配对记录（供未来过滤）+ 返回剩余项；主记录在无剩余项时自动收口（有确认过则 accepted 否则 rejected）。
4. UI（TaskDetailPanel 附件区上方）：条件渲染（mode≠off && features.related）；列表项 = 标题 + 理由 + 摘录 + [关联]/[不相关]；全部处理完显示「已处理 N 条，M 条已关联」；空候选/降级文案。
5. 幂等与边界：同一任务重复请求 dismiss 旧 pending（同切片 3 语义）；确认关联不覆盖用户手工链接。

## Acceptance Criteria

- [ ] 推荐仅展示为独立区；无任何自动写 EntityLink 行为（单测：仅 request 不产生 link）
- [ ] 每条建议含理由与摘录（来源可核）；候选池仅来自本地 FTS + 过滤规则（单测：敏感/已关联/已拒绝均不进候选）
- [ ] 模型编造的标题（不在候选内）被丢弃（单测）
- [ ] [关联] 逐条写 related 链接且幂等；[不相关] 后同任务再请求不出现该配对（单测）
- [ ] AI 关闭/开关关闭无该区；provider 失败降级文案不阻塞详情面板
- [ ] cargo test / pnpm test / build 全绿；docs/ai-assist.md 更新

## 明确不做（本切片）

- 历史任务作为候选（先记忆+剪贴板；任务↔任务推荐属后续）
- 向量/语义相似（Wave C 语义检索落地后自然增强）
- 记忆页反向推荐（本切片只做任务侧入口）
- 推荐理由的自动学习/调权（仅记录拒绝）
