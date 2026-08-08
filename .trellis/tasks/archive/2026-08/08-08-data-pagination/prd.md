# 数据层分页（v1.2.1）

## Goal

为任务、记忆、剪切板三类列表查询引入统一分页协议，避免万级数据下一次性 IPC 全量传输与前端全量渲染，满足 `docs/next-iteration-roadmap.md` §4.1 的 v1.2.1 收尾目标。

**用户价值**：数据量增长后，列表首屏仍快速可用；滚动加载不重复、不遗漏；筛选条件变化后总数与条目一致。

## Background

- 路线图 §4.1 将此项标为 **P1**，是 v1.2.1 唯一未落地的硬伤。
- 8 月功能差距审计（`gap-audit.md`）确认：`query_tasks`（`tasks.rs:540-607`）与 `memory query`（`memories.rs:150-192`）无 `LIMIT`，全量返回；剪切板已有 `limit`（默认 200，`clipboard.rs:362`）但无 `offset` / `total` / `hasMore`。
- 前端 `TasksPage`（`TasksPage.tsx:116-128`）、`MemoryPage`（`MemoryPage.tsx:280-288`）、`ClipboardPage` 均一次性渲染查询结果。

## Requirements

### R1 统一分页协议

- 新增通用分页字段：`limit`（默认 200，上限 1000）、`offset`（默认 0）。
- 查询响应统一为 `{ items, total, hasMore }`（camelCase），替代当前 `Vec<T>` 直返。
- `TaskQuery`、`MemoryQuery`、`ClipboardQuery` 均增加可选 `limit` / `offset`；`task_smart_list` 同步支持分页参数。

### R2 后端分页实现

- `TaskService.query_tasks`：在现有动态 SQL 末尾追加 `LIMIT ? OFFSET ?`；用同条件 `COUNT(*)` 子查询或等价方式返回 `total`；`hasMore = offset + items.len() < total`。
- `MemoryService.query`：同上。
- `ClipboardService.query`：在现有 `LIMIT` 基础上补 `offset`、`total`、`hasMore`。
- `TaskService.smart_list`：内部分页或复用 `query_tasks` 分页路径，行为与清单视图一致。
- 默认 `limit=200` 保持与剪切板现有一致；未传分页参数时行为与「第一页 limit=200」等价（非破坏性默认值）。

### R3 前端列表接入

- `TasksPage`：首屏加载第一页；列表底部「加载更多」或滚动触底加载下一页；筛选/智能列表/标签/保存视图切换时重置 `offset` 并替换列表（非追加）。
- `MemoryPage`：同上（含搜索防抖后的查询重置）。
- `ClipboardPage`：同上；保留现有 `limit: 300` 调用意图，改为走统一协议（可保留较大 limit 或改为分页加载）。
- React Query `queryKey` 纳入 `offset`；加载更多用独立 mutation 或 `fetchNextPage` 模式追加 `items`。

### R4 兼容与范围

- IPC / `client.ts` 类型同步更新；所有 `taskQuery` / `memoryQuery` / `clipboardQuery` / `taskSmartList` 调用点编译通过。
- **不在本期**：`task_today`（`TodayTasks` 按逾期/今日/已完成分组，单日数据量通常可控）、全局搜索（已有独立 `limit`）、提醒列表。

## Out of Scope

- Cursor 游标分页（offset 足够满足桌面本地场景；后续数据量极大再评估）。
- 虚拟滚动库引入（先「加载更多」即可）。
- 任务页内搜索（路线图 §4.5，独立任务）。
- 后端性能基准自动化（手动验收即可）。

## Acceptance Criteria

- [ ] AC1 `task_query` / `memory_query` / `clipboard_query` / `task_smart_list` 返回 `{ items, total, hasMore }`；默认 limit=200。
- [ ] AC2 后端单测：空结果、末页、`hasMore=false`、筛选条件下 `total` 与 `items` 一致（至少 tasks + memories 各 1 组）。
- [ ] AC3 `TasksPage` / `MemoryPage` / `ClipboardPage` 支持加载更多；切换筛选后不出现重复项或遗漏（同一筛选下 `total` 与可见条数一致）。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。
- [ ] AC5 万级 mock 数据下（可用测试夹具或 seed），首屏仅传输 ≤200 条（IPC 载荷可观测或通过单测断言 LIMIT）。

## Key Decisions

| 决策 | 选择 | 理由 |
| --- | --- | --- |
| 分页方式 | offset + limit | 实现简单，与 SQLite 原生契合；本地桌面无深分页性能压力 |
| 默认 limit | 200 | 与剪切板现默认一致；路线图示例值 |
| 响应形态 | 新 `PagedResult<T>` 包装 | 一次 IPC 返回 total，UI 可展示「共 N 条」 |
| task_today | 本期不改 | 按日分组，规模可控；避免改动 Today 复合结构 |

## Risks & Deferred

- **IPC 破坏性变更**：所有调用点需同步改型；风险可控，项目内 grep 可穷尽。
- **COUNT 查询成本**：大表 COUNT 可能慢；可接受（本地 SQLite，且仅列表页）；后续可加缓存。
- **智能列表与清单视图 key 不一致**：加载更多时必须用同一 query 参数集。
