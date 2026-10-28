# 设计：周回顾处理模式

## 1. 架构

新组件 `src/features/weekly-review/ProcessingSession.tsx`，由 `WeeklyReviewPage.tsx` 挂载。会话与统计沿用既有 `weeklyReviewStart/Complete`，**零后端改动**。

## 2. 队列编排

- 进入处理模式时以当前 `WeeklyReviewSnapshot` 冻结队列：
  `queues = [overdue, inboxUnprocessed, waitingFollowUp, staleActive]`（各元素带队列名标签），flat 为出牌序列。
- 状态：`queueIndex`（当前条）、`actions: ActionKind[]`（已出牌记录）。跳过的条目仍占位计数。
- 队列动态性：v1 用进入时快照（不随外部变更重排）；出牌前以 `selectedTaskQuery` 同款方式校验任务仍为 todo，已完结则自动跳过并提示。

## 3. 四键动作映射（全部既有 IPC）

| 键 | 动作 | IPC | 备注 |
| --- | --- | --- | --- |
| 1 | 完成 | `taskComplete` | recurring 既有语义 + 撤销 toast |
| 2 | 延期 | `taskSetDefer(id, 明天)` | v1 固定 +1 天，后续可在任务页改 |
| 3 | 转等待 | `taskSetWaiting(id, null, null)` | 标记等待态，waiting_for 留空后续补 |
| 4 | 跳过 | 无 IPC | 仅推进指针 |

- 每次动作后 `invalidateQueries(["tasks"]) + ["weekly-review"]`（沿用页面既有模式）。
- 键盘监听：处理模式开启时 `window.addEventListener("keydown")`，`1/2/3/4` 出牌、`Esc` 退出（保留会话）；输入框聚焦时短路。

## 4. 统计

- 纯函数 `processingSummary(actions: ActionKind[]): ProcessingSummary`（`{completed, deferred, waiting, skipped, total}`）放 `features/weekly-review/processingSummary.ts` + vitest。
- 结束（队列清空或手动「完成回顾」）把 summary 写入既有 `weeklyReviewComplete(sessionId, { summary })`。

## 5. UI 骨架

- 入口：回顾页头部「开始处理」按钮（四队列全空则置灰 + tooltip）。
- 出牌卡：进度 `第 N / M 条 · <队列名>`、标题/清单/逾期天数、说明摘录、四键按钮（快捷键标注）、Esc 退出。
- 结束卡：统计 + 「完成回顾」按钮（走既有 complete 流程）。

## 6. 测试

- `processingSummary` vitest（空/混合/全跳过）。
- 队列编排纯函数 `flattenQueues(snapshot): ProcessingItem[]`（同文件导出）+ vitest（顺序、空队列跳跃）。
- `pnpm typecheck` 全绿；UI 流程人工矩阵（四键各一、Esc 中断、统计核对）。
