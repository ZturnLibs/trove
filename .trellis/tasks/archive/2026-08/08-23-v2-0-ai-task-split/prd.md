# v2.0 AI 任务拆分（切片 7）

> 父任务：`08-19-v2-0-ai-assist-roadmap`。依赖切片 1（管线）+ 切片 6（检查项模型，已交付）。
> §9.3 对应：任务拆分——「根据任务说明生成候选检查项；默认生成一层检查项，不自动创建多个带提醒的任务；用户可以编辑、删除或全部拒绝；**原任务内容不被模型直接覆盖**」。

## Goal

任务详情检查项区新增「AI 拆分」：模型根据任务标题+说明生成 3–8 条候选检查项（每条附原文依据片段），用户勾选写入既有 checklist；不创建任务、不改原任务任何字段。

## Requirements

1. 模型（`AIFeature::Split.prompt_template()` 开放）：
   - 输入：任务 title + notes（最小上下文）；
   - 输出：`{"items":[{title:检查项内容≤200字, detail:null, dueDate:null, dueTime:null, ambiguous:true, sourceExcerpt:任务原文片段}]}`；
   - **防编造依据**：sourceExcerpt 必须是任务 title+notes 的连续子串（服务端校验，配不上的条目丢弃）。
2. 服务：
   - `request_split(task_id, tasks)`：dismiss 旧 pending（同任务）→ 完成冻结校验 → 空 notes 且短 title → None（无可拆内容，诚实空态）→ 管线 → 子串校验 → pending；
   - `apply_split(input, tasks)`：pending 门禁 + indices 校验 → 逐项 `checklist_add`（内容=模型 title，走既有校验/上限/索引）→ accepted，返回检查项列表；重复 apply 拒绝。
3. UI（ChecklistSection 内）：
   - 「AI 拆分」按钮（mode≠off && features.split && !frozen && total<50）；
   - 草稿区：每项 checkbox（默认全不勾）+ 内容 + 原文依据（等宽小字）+ [添加选中][都不合适]；
   - 添加成功后 invalidate checklist；上限 50 命中时报既有温和文案；
   - 降级/空态文案不阻塞检查项手动操作。
4. 设置页 split 开关「已开放」。

## Acceptance Criteria

- [x] 只生成检查项：不建任务、不改原任务 title/notes（单测：request+apply 前后任务字段零变化）
- [x] 每条草稿显示原文依据；依据不是任务原文子串的条目被丢弃（单测）
- [x] 空说明/短标题不调用 provider（单测零调用）
- [x] completed 任务入口冻结（单测）
- [x] apply 后写入检查项且可撤销（删除）；重复 apply 拒绝（单测）
- [x] AI 关闭/开关关闭时无入口；provider 失败降级文案
- [x] cargo test / pnpm test / build 全绿；docs/ai-assist.md 更新

## 明确不做（本切片）

- 生成带提醒/日期的子任务（§9.3 明确禁止）
- 检查项内容二次编辑（写入后用检查项行内编辑）
- 记忆/剪贴板侧拆分
