# v2.0 每日工作建议（切片 5）

> 父任务：`08-19-v2-0-ai-assist-roadmap`。依赖切片 1（管线）。
> §9.3 对应：每日工作建议——「根据截止时间、优先级、等待状态、今日重点和历史延期情况提出候选事项；给出简短选择理由；**用户决定是否加入今日重点以及顺序**；不依赖日历空闲时间，不自动生成时间块」。
> 现有基础：`today_smart_sort`（确定性排序+固定理由）、今日重点（focus add/remove/reorder + 撤销栈）。

## Goal

今日页新增「今日工作建议（AI）」卡：打开/点击时，AI 从**本地计算的今日候选池**（逾期+今日到期+今日提醒关联，排除已在今日重点/等待中）中挑选 1–3 项作为「建议加入今日重点」，每项附理由；用户逐项 [加入重点] 或 [跳过]；顺序由用户在重点区拖拽决定（AI 不排程、不自动加入）。

## Requirements

1. 候选池（确定性，无模型）：`today_tasks()` 的 overdue + due_today（Todo 状态）；排除：已在 focus、waiting_follow_up；每项附**确定性特征行**（到期日、优先级、延期次数、提醒时间）。
2. 模型职责（`AIFeature::Suggest.prompt_template()` 开放）：从候选特征挑 1–3 项 + 每项一句话理由；title 必须与候选完全一致（回配同切片 4）；**理由须引用确定性特征**（prompt 约束），不编造事实。
3. 服务：
   - `request_daily_suggest(tasks)`：dismiss 旧 pending（source=`daily`）→ 候选（≤15，特征行）→ 空 None → 回配 → pending 记录；
   - 复用既有 `dailyFocusAdd`（前端直调），建议侧只管记录：`decide`（accepted/dismissed）+ 单项 [跳过] 落 dismissed 配对（同任务当天不再推，简化：记录在被跳过配对的 rejected_pair 池，供本日过滤）。
4. UI（TodayPage，智能排序说明附近或重点卡上方）：
   - 条件渲染（mode≠off && features.suggest && 候选非空）；
   - 列表项：标题 + AI 理由 + 特征徽标（到期/优先级/延期）+ [加入重点][跳过]；
   - 加入 → `dailyFocusAdd`（既有撤销栈）+ 项从列表消失；跳过 → dismissed 配对；
   - 全部处理完或无候选 → 卡片隐藏（不占位）；provider 失败 → 降级一行文案。
5. 时效：source=`daily`、当日有效；跨天请求自动 dismiss 昨日 pending（created_at 非今日的 pending 全部收口）。

## Acceptance Criteria

- [x] AI 不改任务任何字段、不自动加入重点（单测：仅 request 后 focus 列表不变）
- [x] 候选仅来自本地确定性池（排除 focus/waiting，单测）；AI 理由基于候选特征（prompt 固化）
- [x] 编造标题被回配丢弃（单测，同切片 4 机制）
- [x] [加入重点] 走既有 dailyFocusAdd + 撤销栈；[跳过] 后当日同项不再出现（单测）
- [x] 跨天自动收口旧 pending（单测：伪造昨日 created_at 后请求先 dismiss）
- [x] AI 关闭/开关关闭/无候选时今日页零 AI 痕迹；provider 失败不阻塞今日页
- [x] cargo test / pnpm test / build 全绿；docs/ai-assist.md 更新

## 明确不做（本切片）

- 自动加入重点/自动排序重点（顺序用户定）
- 时间块/日历/空闲时间
- 建议“新建任务”（只建议既有任务）
- 跨日记忆化偏好（跳过仅当日过滤）
