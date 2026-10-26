# 设计：任务分组（分组 → 清单两级结构）

## 1. 目标与范围

在 `task_lists`（清单）之上引入轻量「分组」实体 `task_list_groups`，侧边栏呈现两级树；今日页与任务页支持按分组过滤。需求与用户决策见 `prd.md`。

**范围外**：颜色/图标、记忆/剪切板分组、托盘过滤、NL 语法路由、跨分组批量操作。

## 2. 数据模型（Migration 0023）

```sql
CREATE TABLE task_list_groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  sort_order REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

ALTER TABLE task_lists ADD COLUMN group_id TEXT REFERENCES task_list_groups(id);
CREATE INDEX idx_task_lists_group ON task_lists (group_id, deleted_at, sort_order);
```

- 与 `task_lists` 同构（软删除 + revision 乐观并发），沿用既有列命名与类型风格。
- 清单→分组为可空外键，NULL = 未分组；**不做** FK ON DELETE 级联（删除分组走应用层解散，见 §4）。
- **折叠状态不进数据库**：分组折叠是视图偏好，走前端 localStorage；与子任务展开状态（DB 持久化）刻意不同——分组折叠高频且纯客户端，避免每次折叠触发 UPDATE/revision。
- Schema 断言：`infrastructure/db/mod.rs` MIGRATIONS 追加 `(23, include_str!(...0023_task_list_groups.sql))`，三处断言 22 → 23。

## 3. 领域模型（`src-tauri/src/domain/task.rs`）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListGroup {
    pub id: EntityId,
    pub name: String,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

pub struct TaskList { /* ...现有字段... */ pub group_id: Option<EntityId> }

pub struct TaskQuery { /* ...现有字段... */ pub list_group_id: Option<EntityId> }

pub struct CreateListInput / UpdateListInput { /* ... */ pub group_id: Option<EntityId> }
// UpdateListInput.group_id 语义：Some(x)=归入组 x；需要「移出分组」时用显式
// set_group(false) 标志位或包装枚举，见 §4.2 —— 以现有 UpdateTaskListInput 的
// Option 字段语义为准，若其 None=不改，则新增 Option<bool> clear_group 字段。
```

侧边栏聚合类型（新，放 domain/task.rs）：

```rust
pub struct TaskListGroupOverview {
    pub groups: Vec<TaskListGroupNode>,       // 按 sort_order
    pub ungrouped: Vec<TaskListSummary>,      // 未分组自定义清单
    pub inbox: TaskListSummary,               // 固定顶部（沿用现有 kind=inbox）
}
pub struct TaskListGroupNode { pub group: TaskListGroup, pub lists: Vec<TaskListSummary>, pub open_count: u64 }
pub struct TaskListSummary { pub id: EntityId, pub name: String, pub kind: ListKind, pub group_id: Option<EntityId>, pub open_count: u64 }
```

## 4. 后端服务（新文件 `src-tauri/src/application/list_groups.rs`）

以 `impl TaskService` 扩展块实现（同 crate 允许跨文件 inherent impl；若 TaskService 字段非 pub(crate) 则调整可见性，不新建服务结构体、不改 AppState 装配）。

### 4.1 分组 CRUD

- `list_group_create(name) -> TaskListGroup`：名称非空去重（与现存活跃组重名 → `Validation` 错误）；`sort_order = MAX+1`。
- `list_group_rename(id, name) -> TaskListGroup`：revision +1（乐观并发同清单）。
- `list_group_reorder(ordered_ids: Vec<EntityId>)`：按给定顺序重写 `sort_order = 0..n`；校验 id 集合完整。
- `list_group_delete(id) -> ListGroupDeleteUndo`：
  - 事务内：组内清单 `group_id = NULL`（**解散，任务与清单都不删**）→ 组软删除；
  - 返回 `ListGroupDeleteUndo { group: TaskListGroup, moved_list_ids: Vec<EntityId> }` 供撤销（恢复组 + 回链清单），复用清单删除已有的撤销注册机制（`tasks.rs:70-289` 模式）。

### 4.2 清单归组

- `update_task_list`（现有）扩展：允许设置 `group_id`；校验：组存在且活跃；**`kind=inbox` 的清单拒绝归组**（`Validation` 错误）。
- 「移出分组」= `group_id` 置 NULL，按现有 Update 输入语义选择建模（None=不改则补 `clear_group` 标志）。

### 4.3 侧边栏聚合

- `task_list_overview() -> TaskListGroupOverview`：两条查询（组 + 清单）+ 一条 `COUNT` 聚合（`tasks WHERE status='todo' AND deleted_at IS NULL GROUP BY list_id`），内存拼装；一次 IPC 供侧边栏全量渲染。

### 4.4 过滤接入

- `build_task_filters()`（tasks.rs:1203）：`TaskQuery.list_group_id` → `EXISTS (SELECT 1 FROM task_lists lg WHERE lg.id = t.list_id AND lg.group_id = ?)`（或等价 JOIN；注意 tasks 查询已 JOIN task_lists 时直接加 `l.group_id = ?`）。
- `today_tasks(list_group_id: Option<...>)`：参数透传进内部过滤；None = 现状（全部）。SmartList/counts 不动。

## 5. IPC 命令层（`commands/mod.rs` + `lib.rs` 注册）

| 命令 | 签名 | 说明 |
|---|---|---|
| `task_list_group_create` | `(name) -> TaskListGroup` | |
| `task_list_group_rename` | `(id, name) -> TaskListGroup` | |
| `task_list_group_reorder` | `(ordered_ids) -> ()` | |
| `task_list_group_delete` | `(id) -> ListGroupDeleteUndo` | 解散 + 可撤销 |
| `task_list_overview` | `() -> TaskListGroupOverview` | 侧边栏全量数据 |
| `task_list_update` | 扩展输入（group_id / clear_group） | 归组/移出 |
| `task_query` / `task_query_tree` | `TaskQuery.list_group_id` | 任务页分组过滤 |
| `today_tasks` | 加可选 `listGroupId` 参数 | 今日 chips |

- 变更类命令后 emit 现有 `domain://changed` 事件并 invalidate 前端查询（沿用清单命令做法）。

## 6. 前端

### 6.1 IPC 绑定（`src/ipc/client.ts`）

- 新类型 `TaskListGroup` / `TaskListGroupNode` / `TaskListSummary` / `TaskListGroupOverview`；`TaskList` 加 `groupId: string | null`；`TaskQuery` 加 `listGroupId?: string`。
- 新方法：`taskListGroupCreate/Rename/Reorder/Delete`、`taskListOverview()`；`todayTasks` 加可选参数。

### 6.2 侧边栏（`src/app/layouts/MainShell.tsx`）

- 「任务」区块改为数据驱动：`taskListOverview()`（TanStack Query，15s refetch 与现有 taskCounts 一致）。
- 结构：收件箱（不动，顶级）→ 任务区 = 各分组（可折叠，名称行带 open_count 徽标与 chevron）→ 未分组清单平铺 → 分组/清单 hover 出现右键能力。
- 交互：
  - 分组名点击 → `navigate('/tasks/group/:groupId')`；箭头点击 → 折叠（localStorage `sidebar.groupCollapsed`）。
  - 清单点击 → `navigate('/tasks/:listId')`。
  - dnd-kit：清单拖入分组（`updateTaskList(groupId)`）、拖出（clear）、分组上下拖拽排序（`taskListGroupReorder`）。收件箱不参与 DnD。
  - 右键菜单：分组（重命名/删除/上移下移）、清单（重命名/移动到分组/删除——复用现有清单删除三选一交互）、任务区空白/标题（新建分组/新建清单）。
  - 分组重命名行内编辑；删除走现有确认 + 撤销 toast 模式。

### 6.3 路由（`AppRouter.tsx` + `TasksPage.tsx`）

- `/tasks/:listId`：TasksPage 从 route param 初始化 `listId` 过滤（现状忽略）；param 与内部筛选状态单向同步（URL 变 → 状态变；页内切清单 → replace 导航保持 URL 真实）。
- 新增 `/tasks/group/:groupId`：TasksPage 以 `listGroupId` 过滤（下拉「清单」显示为该组名）。

### 6.4 任务页（`TasksPage.tsx`）

- 清单筛选下拉改为 分组 optgroup 分层（全部/收件箱/各组/未分组清单）。
- 「全部清单」视图新增「按清单分区显示」开关（存入现有视图状态持久化形状，`TasksPage.tsx:120-131`）：开启时按清单用 `TaskGroup`（TaskLayout.tsx:53-86）粘性分节渲染，组内沿用现有排序。
- 「移动到清单」（详情面板 TaskDetailPanel.tsx:278-284 + 右键）：菜单按 分组→清单 分层；加「移出到未分组」不适用（清单无分组也合法），仅按组分层展示。
- 空状态引导：`groups.length === 0 && lists.length <= 1` 且 localStorage 未标记时，列表顶部渲染一次性引导条：「一键创建 工作 / 个人 分组」→ 批量调 `taskListGroupCreate ×2` + 建 2 个清单并归组，写 localStorage 标记。

### 6.5 今日页（`TodayPage.tsx`）

- 顶部过滤 chips：`全部` + 各分组 + `未分组`；数据经 `todayTasks({ listGroupId })` 重新查询；五区块统一过滤。
- 选择持久化 localStorage（`today.groupFilter`），默认全部。
- 托盘 tray-today 窗口不传参（保持全部）。

### 6.6 快速捕获（`QuickWindow.tsx`）

- 任务捕获模式加目标清单选择器（默认 `收件箱`；选项来自 `taskListOverview`，按分组分层）。
- 不记忆上次选择（保持收件箱默认，防止误归档）；记忆功能 v1 范围外。

## 7. 测试计划

### 7.1 后端（`application/list_groups.rs` 内 `#[cfg(test)]`，沿用 `open_service()`/`test_db()` 模式）

1. 迁移：db/mod.rs 断言 22→23；迁移后旧库打开正常（空组表、清单 group_id 全 NULL）。
2. CRUD：创建（sort_order 追加、重名校验）、重命名（revision+1）、排序（全量重写）。
3. 解散删除：组软删、组内清单 group_id=NULL、任务不动、返回 moved_list_ids；undo 恢复组与归链。
4. 归组校验：inbox 拒绝归组；不存在/已删组拒绝。
5. `task_list_overview`：分组→清单嵌套正确、open_count 只数 todo 未删、未分组/收件箱归类正确。
6. `TaskQuery.list_group_id` 过滤：query_tasks 与 query_tree 结果一致、跨清单聚合正确。
7. `today_tasks` 带/不带 listGroupId。

### 7.2 前端

- `pnpm typecheck`；`pnpm test:unit`（若新增纯函数：overview→侧边栏树 的映射抽纯函数并加 vitest）。
- 手工矩阵：老库升级（备份后真实 DB 打开）、拖拽归组/移出、分组过滤今日页、快速捕获选清单、撤销 toast、`/tasks/:listId` 直达。

## 8. 兼容性与风险

- **旧数据零迁移**：新表空、`group_id` 全 NULL，所有现有行为不变；`saved_views` 的 filter_json 不透明，后续如保存分组过滤自然兼容。
- **降级兼容**：旧版应用显式列名 SELECT/INSERT，不感知新列（清单 INSERT 需确认列清单写法；若为 `INSERT INTO task_lists (...)` 显式列则安全——实施时确认，必要时在新列上用 `DEFAULT NULL` 已满足）。
- **性能**：侧边栏一次聚合 IPC；`group_id` 有索引；任务过滤走 EXISTS，任务量级（个人工作台）无风险。
- **风险点**：dnd-kit 侧边栏嵌套拖拽复杂度最高 → 优先保证右键「移动到分组」可用，拖拽为增强；TasksPage 路由参数与内部状态同步要防循环 setState。

## 9. 文档更新

- `docs/ui-layout-interaction.md`：侧边栏小节补充两级任务分组说明。
- `docs/development-roadmap.md`：若「清单/分组」在规划或暂不包含清单中，标注完成。
