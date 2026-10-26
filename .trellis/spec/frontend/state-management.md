# State Management

> How state is managed in this project.

---

## Overview

- Server state: TanStack Query（服务端状态的唯一通道，禁止手动镜像进 useState）。
- URL state: react-router 路由参数（需要直达/刷新还原的过滤视图）。
- 视图偏好（折叠、过滤选择、一次性引导标记）: localStorage。
- 领域数据: SQLite（后端持有 + revision 乐观并发，前端只是查询缓存）。

---

## State Categories

| 类别 | 存放 | 判据 | 例 |
| --- | --- | --- | --- |
| Server state | TanStack Query | 来自 IPC/后端 | `["task-list-overview"]` |
| URL state | 路由参数 | 需要直达/刷新还原 | `/tasks/:listId`、`/tasks/group/:groupId` |
| 视图偏好 | localStorage | 高频、纯客户端、丢失无碍 | `sidebar.groupCollapsed`、`today.groupFilter` |
| 领域数据 | DB（revision 乐观并发） | 跨会话必须一致、算业务数据 | 分组、清单、任务 |

**视图偏好不进 DB**：分组折叠状态（task-list-groups）刻意走 localStorage——折叠是高频纯视图操作，每次折叠触发 UPDATE + revision 会放大写入并污染乐观并发语义。对比：子任务展开状态算业务数据，进 DB。新增视图偏好时先按上表判据选列，不要默认落库。

localStorage 读写一律 try/catch 吞掉 quota/private-mode 异常（全仓既有模式：`DeferPicker.tsx`、`TaskSidebarTree.tsx`、`TodayPage.tsx`）。

## URL ↔ Filter State Sync

过滤型页面（`TasksPage.tsx`）的 URL 参数与内部筛选状态只做**单向同步**：

- URL → 状态：`useParams()` + `useEffect`，进入页面或参数变化时写入 state。
- 状态 → URL：仅页内主动切换清单/分组时 `navigate(..., { replace: true })`，保持地址栏真实。
- 禁止再写一条 state → URL 的 effect 回写——两条 effect 互相触发会循环 setState（task-list-groups design §8 标记的头号风险）。

---

## When to Use Global State

<!-- Criteria for promoting state to global -->

(To be filled by the team)

---

## Server State

- queryKey 必须包含全部过滤参数（见 `TasksPage.tsx` 的 `treeQueryKey`），漏参数表现为「切筛选命中旧缓存」。
- 变更类 mutation onSuccess → `invalidateQueries` 对应 key；后端命令同时 emit `domain://changed` 事件（沿用清单/分组命令做法）。
- 侧边栏类常驻聚合数据用 `refetchInterval: 15_000`，与现有 taskCounts 对齐（`TaskSidebarTree.tsx`）。

## Common Mistakes

- **URL↔state 双向 effect 循环**：过滤页只保留 URL→state 单向 + 页内 replace（见上节）。
- **裸 localStorage 读写**：private mode / quota 抛异常会炸掉整个渲染路径，必须 try/catch。
- **过滤参数漏出 queryKey**：缓存串页，表现为切换筛选后显示旧数据。
- **乐观补丁判定条件 ≠ 渲染分流条件**：补丁改写字段后，若分流/分组函数不校验该字段对应的终态（如 board 看板按 `boardStage` 分流却忽略 `status`），行会滞留原列等 refetch——乐观更新空转。分流纯函数必须把「哪些行属于哪列」的判据与补丁会改变的字段对齐（board 看板 P2 教训：`splitBoardColumns` 需校验 `status`）。
