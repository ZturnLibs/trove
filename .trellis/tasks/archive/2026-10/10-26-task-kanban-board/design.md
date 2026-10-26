# 设计：任务看板视图

## 1. 目标与范围

独立 `/board` 页：待办 / 进行中 / 已完成（今日）三列，dnd-kit 拖拽推进。需求与决策见 `prd.md`（R1 阶段列 / R2 全局+chips / R3 独立路由 / R4 拖拽语义）。

**范围外**：列内手动排序、自定义列、WIP、批量拖拽、waiting/deferred 上板。

## 2. 数据模型（Migration 0024）

```sql
ALTER TABLE tasks ADD COLUMN board_stage TEXT;
```

- 唯一合法非空值 `'doing'`；NULL = 待办。应用层校验（写路径枚举解析），不加 CHECK 约束（与既有迁移风格一致）。
- **不加索引**：查询走既有 `status` 条件（`build_task_filters`），stage 由客户端分流，个人量级无风险。
- `infrastructure/db/mod.rs`：MIGRATIONS 追加 `(24, include_str!(...0024_task_board_stage.sql))`；三处 schema_version 断言 23 → 24。
- 纯增量，revert 代码后旧库可打开（显式列清单惯例，09-08 已确认全部 INSERT 均显式列名——**新增列后需复查 tasks 的 INSERT/SELECT 列清单**，`tasks.rs:2504-2506` 的 SELECT 列清单必须加 `t.board_stage`）。

## 3. 领域模型（`domain/task.rs`）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskBoardStage { Doing }   // 仅一态，可扩展

pub struct Task { /* ... */ pub board_stage: Option<TaskBoardStage> }
```

- `as_str/parse`：`"doing"` ↔ `Doing`；parse 供写路径校验，非法值 → `Validation` 错误。
- SELECT/row 映射：`row.get::<_, Option<String>>`（nullable 列惯例，spec Pitfalls）。

## 4. 服务层（`application/tasks.rs` 扩展）

```rust
pub fn set_task_board_stage(&self, id: EntityId, stage: Option<TaskBoardStage>) -> Result<Task, DomainError>
```

- 校验：任务存在且未删；`status == Todo`（否则 `Validation`「仅待办任务可切换阶段」）。
- 写入：`UPDATE tasks SET board_stage = ?1, updated_at, revision+1`；`None` → 置 NULL（回待办）。
- **完成路径不动 `complete_task` 签名**，但在其事务内补一句 `board_stage = NULL`（落实 R1 不变式：完成即出阶段）。`uncomplete_task` 不恢复 stage（回待办列）。
- 归档路径（`archive_task`）同样置空——防脏值。

## 5. IPC（`commands/mod.rs` + `lib.rs`）

| 命令 | 签名 | 说明 |
|---|---|---|
| `task_set_board_stage` | `(id, stage: Option<String>)` → `Task` | `"doing"`/`null`；emit `domain://changed` |

- 查询零新增命令：复用 `task_query` 两次调用（见 §7）。

## 6. 前端绑定（`src/ipc/client.ts`）

- `Task` 加 `boardStage: "doing" | null`；新方法 `taskSetBoardStage(id, stage: "doing" | null)`。

## 7. BoardPage（新文件 `src/features/board/BoardPage.tsx`）

- **数据**（TanStack Query，key 含 listGroupId）：
  - `q1 = taskQuery({ status: "todo", listGroupId })` → 客户端分流：`boardStage === "doing"` → 进行中列，否则待办列（分流抽纯函数 `splitBoardColumns` + vitest，沿用 `listGroupTree.ts` 纯函数模式）。
  - `q2 = taskQuery({ status: "completed", completedSince: <今天>, listGroupId })` → 已完成列。
  - `taskListOverview()` → 分组 chips 数据源（复用 09-08）。
- **chips**：`全部 | 各分组 | 未分组`，交互与持久化复刻今日页（localStorage key `board.groupFilter`，try/catch 惯例，spec frontend/state-management.md）。URL `?group=` ↔ chips 单向同步（URL → 状态 useEffect；页内切换 replace——spec 单向同步规约）。
- **三列布局**：横向 flex，列 min-w 固定、容器横向滚动；列头 = 名称 + 计数徽标。
- **DnD**：一个 `DndContext` + 每列 `useDroppable`（TaskSidebarTree 模式）；卡片 `useDraggable`；PointerSensor + distance 约束。`onDragEnd` 按目标列分发：
  - → 进行中：`taskSetBoardStage(id, "doing")`
  - → 待办（来自进行中）：`taskSetBoardStage(id, null)`；（来自已完成）：`taskUncomplete(id)`
  - → 已完成：`taskComplete(id)`——复用现有完成 mutation（含 recent-actions 撤销 toast）；recurring 生成下一期落待办列（既有语义，天然正确）。
  - 乐观更新 + `invalidateQueries(["tasks"])`；失败回滚。
- **卡片**（新 `BoardCard.tsx`，精简于 TaskRow）：标题、清单名、到期日（逾期红）、优先级点、子任务计数徽标；点击 → 打开 `TaskDetailPanel`（复用）；右键 → 复用现有任务右键菜单组件。
- **路由**：`AppRouter.tsx` 加 `/board`；`MainShell.tsx` 导航数组加「看板」项（图标 `KanbanSquare`，lucide）。

## 8. 兼容与风险

- 旧数据零迁移：`board_stage` 全 NULL = 待办，行为不变。
- **风险 1**：`tasks.rs:2504` SELECT 列清单漏加 `board_stage` → 映射错位。实施时该行必须同步改，`map_row` 测试覆盖。
- **风险 2**：拖入完成列走 `complete_task`，其子任务级联/recurring spawn 均为既有语义，看板不得绕行自写 status UPDATE。
- **风险 3**：三列查询是两次 IPC 而非一次聚合——看板页量级可接受（与侧边栏 15s refetch 同策略），不做新聚合命令。

## 9. 测试

### 后端（`application/tasks.rs` `#[cfg(test)]`）

1. `set_task_board_stage`：置 doing / 清空回待办；`completed`/`archived` 任务拒绝。
2. `complete_task` 后 `board_stage` 为空；`uncomplete_task` 后仍为空（回待办）。
3. 迁移 0024 后旧任务全部 `board_stage IS NULL`；schema 断言 24。
4. recurring 任务带 stage 完成后，spawn 的下一期 `board_stage IS NULL`。

### 前端

- `splitBoardColumns` 纯函数 vitest（todo 分流、completed 列、listGroupId 过滤透传）。
- `pnpm typecheck`、`pnpm test:unit` 全绿。

### 手工矩阵（review 阶段）

拖拽三向、撤销 toast、recurring 拖完成、chips 记忆、`/board?group=` 深链、老库升级。
