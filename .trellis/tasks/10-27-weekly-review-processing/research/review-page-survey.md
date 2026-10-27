# Research：周回顾处理模式（单条出牌）

## 现状锚点（2026-10-27 复核）

### 页面与数据

- `src/features/weekly-review/WeeklyReviewPage.tsx`（429 行）：
  - 会话模型**已存在**：`weeklyReviewStart()` → sessionId，`weeklyReviewComplete(sessionId, ReviewCompleteInput)`（`:145-167`）——处理模式直接挂进既有会话生命周期，无需新后端。
  - 数据源：`weeklyReviewSnapshot()` → `WeeklyReviewSnapshot`（`client.ts:541-556`），四条队列字段齐全：
    - `overdue` / `inboxUnprocessed` / `waitingFollowUp` / `staleActive`（各带 Count）
  - 现有 UI 是 ReviewCard 平铺（`:306` 起），`TaskReviewRow` 只有「完成」一键（`:80-104`）——处理模式是对这些队列的**逐条视图**，不是新数据。
  - `TaskDetailPanel` 已内嵌（`selectedTaskId` → `:194` 查询），出牌卡「查看详情」直接复用。
- 可复用 mutation（全部现成）：`taskComplete / taskUncomplete / taskSetDefer / taskSetWaiting / taskDeleteTree`；撤销走 `useRecentActions`。

## 设计约束（来自产品分析 2026-10-26 会话）

- 处理顺序：逾期 → 收件箱积压 → 等待跟进 → 僵滞活跃（staleActive）；同一队列内按现有快照顺序。
- 四键出牌：完成 / 延期（available_at）/ 转等待 / 跳过（本条不出列，仅跳过）；键盘 `1-4` + `J/K` 不必须（v1 用数字键 + Enter 确认即可）。
- 复理统计：出牌动作计数进 `weeklyReviewComplete` 的 summary（`ReviewCompleteInput.summary: Record<string, unknown>` 兼容）。
- 温和原则（调研报告 §3.4）：无 guilt 文案；跳过不算失败。

## 风险

- 快照是进入回顾时刻的快照——处理过程中任务可能被其他窗口改动：出牌前校验任务 status（乐观 mutation 已有快照回滚模式，见 BoardPage）。
- 队列为空时的空态与「进入处理模式」入口要置灰。

## 待设计阶段决策

- 出牌时是否允许直接编辑标题/清单（引 DetailPanel 即可，倾向不重复造行内编辑）。
- 处理进度是否持久化（会话中断恢复）——`weeklyReview` 会话表结构待查（`weekly_review_sessions`?）。
