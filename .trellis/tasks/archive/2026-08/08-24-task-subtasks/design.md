# 设计：对 task 支持子任务（多层嵌套）

## 1. 目标与范围

在 Trove 中为任务（task）引入**多层嵌套子任务**（真·完整任务，非轻量检查项）。子任务是完整任务，拥有自己的状态/优先级/截止日期/标签/检查项/提醒等，独立参与一切视图（今日/搜索/智能列表/提醒/托盘/CLI/CSV）。

**范围外**：任务依赖、附件、每任务覆盖完成策略、展开状态 localStorage（已确认走数据库）。

## 2. 数据模型（Migration 0022）

### 2.1 `tasks` 表新增列

```sql
ALTER TABLE tasks ADD COLUMN parent_id TEXT REFERENCES tasks(id);
ALTER TABLE tasks ADD COLUMN child_order REAL NOT NULL DEFAULT 0;
CREATE INDEX idx_tasks_parent_active ON tasks (parent_id, deleted_at, child_order);
```

- `parent_id`：自引用外键，NULL = 顶层任务。软删除（`deleted_at`）语义下不依赖 FK 级联。
- `child_order`：**同父兄弟排序**（REAL，类似现有 `sort_order` 但作用域是 parent 内）。
  - `sort_order` 保持「清单内排序」，仅用于顶层任务。
  - 归巢后**保留原 `list_id`**（已确认），因此兄弟排序不能继续用清单作用域的 `sort_order`，需要独立的 `child_order`。
- 索引 `idx_tasks_parent_active` 支撑「查某父任务的所有活跃子任务」与树形聚合。

### 2.2 展开状态表（数据库持久化，已确认 B）

```sql
CREATE TABLE IF NOT EXISTS task_tree_expanded (
  task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  expanded INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL
);
```

- 语义：**记录被折叠的任务**（默认展开）。`task_id` 存在且 `expanded = 0` → 折叠；缺失 → 展开。
- 只有「有子任务」的任务才需要记录；只写入用户显式折叠的任务，避免膨胀。
- 删除任务时随任务级联清除（FK ON DELETE CASCADE；软删除场景由应用层在 `delete_task` 中同步清理）。

### 2.3 Schema 断言

`src-tauri/src/infrastructure/db/mod.rs`：
- `MIGRATIONS` 数组追加 `(22, include_str!("../../../migrations/0022_task_subtasks.sql"))`。
- 三处 `assert_eq!(..., schema_version, 21)` → `22`。

## 3. 领域模型（`src-tauri/src/domain/task.rs`）

### 3.1 `Task` 结构体新增

```rust
pub struct Task {
    // ...现有字段...
    pub parent_id: Option<EntityId>,
    pub child_order: f64,
}
```

### 3.2 `CreateTaskInput` 新增

```rust
pub struct CreateTaskInput {
    // ...现有字段...
    pub parent_id: Option<EntityId>,
}
```

### 3.3 `TaskQuery` 新增（子任务列表查询）

```rust
pub struct TaskQuery {
    // ...现有字段...
    pub parent_id: Option<EntityId>,   // 过滤某父任务的直接子任务（详情面板用）
}
```

### 3.4 新枚举 / 常量

```rust
pub const MAX_SUBTASK_DEPTH: usize = 5;   // 嵌套深度上限（已确认）

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskDeleteDisposition {
    Cascade,   // 级联删除：父任务 + 所有后代
    Promote,   // 仅删除父任务：直接子任务提升为顶层（parent_id = NULL）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTreeExpanded {
    pub task_id: EntityId,
    pub expanded: bool,
    pub updated_at: String,
}
```

### 3.5 校验函数

```rust
pub fn validate_parent_depth(ancestor_chain_len: usize) -> Result<(), DomainError>
// 超过 MAX_SUBTASK_DEPTH → Validation("子任务嵌套最多 5 层")
```

## 4. 设置项（`src-tauri/src/infrastructure/settings/mod.rs`）

`AppSettings` 新增两个开关（全局配置，已确认）：

```rust
#[serde(default = "default_true")]
pub subtask_auto_complete_parent: bool,   // 所有子任务完成 → 父任务自动完成（含反向）
#[serde(default = "default_true")]
pub subtask_cascade_children: bool,       // 父任务完成 → 级联子任务（含反向）
```

- 使用既有 `default_true` 模式，旧 JSON 兼容。
- 前端 `AppSettings` 类型同步新增两个字段（`src/ipc/client.ts`）。

## 5. 后端服务（`src-tauri/src/application/tasks.rs`）

### 5.1 完成聚合（核心逻辑，`complete_task` / `uncomplete_task`）

**`complete_task(id)`**（在现有事务内扩展）：

1. 完成目标任务（现有逻辑：置 completed、清 daily_focus、周期任务 spawn 下一实例）。
2. **若 `subtask_cascade_children` 开启**：递归完成所有活跃后代（`WITH RECURSIVE` 查后代，逐个完成，不触发各自 spawn/focus 重复清理——统一处理）。
3. **若 `subtask_auto_complete_parent` 开启**：沿 `parent_id` 向上走，只要某祖先的**直接子任务全部完成** → 完成该祖先（递归向上，同样不重复触发 series spawn）。

> 注意：级联完成的子任务与自动完成的父任务，其 series spawn 只对「被显式完成的目标任务」执行，避免级联过程中误触发多次周期实例生成（现有 `spawn_next_series_instance` 只在目标任务有 `series_id` 时调用）。

**`uncomplete_task(id)`**（对称反向，已确认双向）：

1. 恢复目标任务为 todo。
2. **若 `subtask_cascade_children` 开启**：递归恢复所有后代为 todo。
3. **若 `subtask_auto_complete_parent` 开启**：沿 `parent_id` 向上走，凡是 `completed` 的祖先 → 恢复为 todo（直到遇到非 completed 祖先停止）。

> 边界：父任务被「手动」完成后，子任务被取消完成 → 父任务也恢复 todo（符合已确认的「向上双向」规则；不区分自动/手动完成）。

**事务性**：聚合改动在同一事务内，任一步失败整体回滚。

### 5.2 归档级联（`archive_task` / `unarchive_task`）

- `archive_task(id)`：归档目标 + 递归归档所有活跃后代（已确认）。
- `unarchive_task(id)`：对称恢复目标 + 后代。

### 5.3 删除处置（`delete_task` 扩展 + 新命令）

- 保留 `delete_task(id)` 语义（叶子任务删除 + 检查项清理 + 搜索索引/关联清理），但**若任务有活跃子任务则拒绝**（`Validation("该任务有子任务，请使用 task_delete_tree")`），防止静默产生孤儿。
- 新方法 `delete_task_tree(id, disposition: TaskDeleteDisposition)`：
  - `Cascade`：软删除目标 + 所有活跃后代（含各任务检查项、`task_tree_expanded` 行），返回删除的任务 id 列表。
  - `Promote`：软删除目标；直接子任务 `parent_id = NULL`（提升为顶层，`list_id` 不变），孙代保持挂在子任务下。
- 两个处置都返回受影响任务 id，供命令层逐一清理搜索索引、关联、展开状态并 emit 事件。

### 5.4 归巢 / 提升（新方法 `set_task_parent`）

```rust
pub fn set_task_parent(&self, id: EntityId, parent_id: Option<EntityId>) -> Result<Task, DomainError>
```

- `parent_id = None`：提升为顶层。`parent_id = NULL`，`child_order = 0`（顶层排序走 `sort_order`，提升后重新分配 `sort_order = MAX(sort_order)+1` 于其清单末尾）。
- `parent_id = Some(p)`：校验：
  1. `p != id`（不能是自己的子任务）；
  2. `p` 不是 `id` 的后代（**循环引用防护**：沿 `p` 的 parent 链向上查是否出现 `id`）；
  3. 嵌套深度校验：`depth(p 的祖先链) + 1 ≤ MAX_SUBTASK_DEPTH`；
  4. `p` 与 `id` 均须活跃（未软删）。
  然后 `parent_id = p`，`child_order = MAX(child_order)+1` 于新兄弟间（追加到末尾）。
- `list_id` 保持不变（已确认归巢不改清单归属）。

### 5.5 兄弟排序（新方法 `reorder_subtasks`）

```rust
pub fn reorder_subtasks(&self, parent_id: Option<EntityId>, ordered_ids: Vec<EntityId>) -> Result<(), DomainError>
```

- 对给定兄弟列表（同一 parent，或 `None` 表示顶层）按序重写 `child_order = 0..n`（顶层写 `sort_order` 沿用现有 `reorder_tasks` 逻辑，但限定同一 parent 作用域）。
- 校验 ordered_ids 均属于该 parent。

### 5.6 树形查询（新方法 `query_tree`）

```rust
pub fn query_tree(&self, query: TaskQuery) -> Result<Vec<Task>, DomainError>
```

- **匹配集 M**：复用 `query_tasks` 的过滤器构建逻辑（提取公共 filter builder），但**不分页**（树形视图整体返回；个人工作台任务量可接受，后续可按需优化）。
- **闭包扩展**：
  - 对 M 中每个任务，向上补全祖先链（`WITH RECURSIVE` 沿 `parent_id` 向上，直到 NULL 或已软删），向下补全全部后代。
  - 祖先/后代**不套用 list/status 等业务过滤器**（保证树形完整）；只排除软删任务。
- 返回去重后的 `Vec<Task>`（含 `parent_id`/`child_order`），前端组装树。
- `query_tasks` 新增 `parent_id` 过滤（直接子任务，按 `child_order` 排序）供详情面板「子任务」区使用。

### 5.7 展开状态（新方法）

```rust
pub fn tree_expanded_list(&self) -> Result<Vec<TaskTreeExpanded>, DomainError>  // 全量（任务量小）
pub fn set_tree_expanded(&self, task_id: EntityId, expanded: bool) -> Result<(), DomainError>
```

- `set_tree_expanded`：`expanded=false` 时 upsert 记录；`expanded=true` 时删除记录（恢复默认展开）。`task_id` 需存在。
- `delete_task` / `delete_task_tree` 中同步删除对应展开记录。

### 5.8 现有方法适配

- `create_task`：支持 `parent_id`。若指定 parent：校验存在、活跃、深度 ≤ 5；`list_id` 缺省时继承父任务清单（未指定 list_id 时）；`child_order = MAX(child_order)+1` 于兄弟间。
- `query_tasks`：`parent_id` 过滤分支 + 排序（有 parent_id 过滤时按 `child_order ASC, created_at DESC`）。
- `TASK_ROW_SELECT` + `map_task_row`：追加 `t.parent_id, t.child_order` 两列（map 索引 20、21）。

## 6. IPC 命令层（`src-tauri/src/commands/mod.rs`）

| 命令 | 变更 |
|---|---|
| `task_create` | `CreateTaskInput.parent_id` 已含，服务层处理；返回 Task 含新字段 |
| `task_query` | `TaskQuery.parent_id` 过滤 |
| `task_query_tree` | **新增**：`query: TaskQuery` → `Vec<Task>` |
| `task_set_parent` | **新增**：`(id, parent_id: Option<EntityId>)` → `Task` |
| `task_reorder_subtasks` | **新增**：`(parent_id: Option<EntityId>, ordered_ids)` → `()` |
| `task_delete_tree` | **新增**：`(id, disposition: TaskDeleteDisposition)` → 返回删除/受影响 id 列表 |
| `task_tree_expanded_list` | **新增**：→ `Vec<TaskTreeExpanded>` |
| `task_set_tree_expanded` | **新增**：`(task_id, expanded)` → `()` |
| `task_complete` / `task_uncomplete` | 服务层聚合已覆盖；命令层对**每个受影响任务** emit `domain://changed` + 更新搜索索引（级联/聚合改动需同步到 UI 与索引） |
| `task_archive` / `task_unarchive` | 同上：对每个受影响任务 emit |
| `task_delete` | 服务层拒绝有子任务的删除；命令层行为不变 |

- 新增命令统一注册到 `invoke_handler`。
- 自动化规则（`maybe_run_automation_*`）：级联/聚合引起的**非显式目标**任务暂不触发自动化规则（范围外，设计注记）；显式目标保持现有触发。

## 7. 前端设计

### 7.1 IPC 绑定（`src/ipc/client.ts`）

- `Task` 类型追加 `parentId: string | null`、`childOrder: number`。
- `AppSettings` 追加 `subtaskAutoCompleteParent: boolean`、`subtaskCascadeChildren: boolean`。
- `CreateTaskInput` 追加 `parentId?: string`；`TaskQuery` 追加 `parentId?: string`。
- 新绑定：`taskQueryTree(query)`、`taskSetParent(id, parentId)`、`taskReorderSubtasks(parentId, orderedIds)`、`taskDeleteTree(id, disposition)`、`taskTreeExpandedList()`、`taskSetTreeExpanded(taskId, expanded)`。
- 新类型：`TaskDeleteDisposition = "cascade" | "promote"`、`TaskTreeExpanded`。

### 7.2 列表视图（`src/features/tasks/TasksPage.tsx`）

**清单视图（`smart === "none"`）改为树形渲染：**

1. 查询改为 `taskQueryTree`（带当前筛选：list/status/priority/tag/search/defer/waiting），返回闭包集。
2. 前端组装树：`childrenMap = Map<parentId|null, Task[]>`，按 `childOrder` 排序；根节点按 `sortOrder`。
3. **渲染**：递归（或扁平化 visible 列表）渲染 `SortableTaskRow`；子任务缩进（`padding-left = depth * 16px`）。
4. **展开/折叠**：有子任务的行显示 chevron；点击调用 `taskSetTreeExpanded`；展开状态从 `taskTreeExpandedList` 读取（默认展开）。初始加载后按状态过滤 visible 行。
5. **进度徽标**：父任务行显示 `已完成子任务/子任务数`（类似 ChecklistBadge，`x/y`）。
6. **拖拽**（dnd-kit）：
   - 拖到某任务行**上（目标行高亮「成为子任务」区）** → `taskSetParent(dragged, targetId)`（归巢）。
   - 拖到**两行间隙** → 同一父节点内重排（`taskReorderSubtasks`）。
   - 拖拽过程中按目标行是否处于「归巢区」渲染 drop indicator。
7. 跨层移动统一走归巢（已确认）；归巢后 invalidate `["tasks"]`。

**智能列表视图**（`smart !== "none"`）：保持现有扁平 `taskQuery` 渲染（子任务独立可见，不树形）。

**删除入口**：任务行/详情删除时，先查询是否 `hasActiveChildren`（前端已有闭包集可判断）；有子任务 → 弹处置确认框（级联删除 / 仅删除父任务），调 `taskDeleteTree`；无子任务 → 直接 `taskDelete`。

### 7.3 详情面板（`src/design-system/patterns/TaskDetailPanel.tsx`）

- 在 `ChecklistSection` 之前新增 **`SubtaskSection`**（新组件 `src/design-system/patterns/SubtaskSection.tsx`，模式参考 ChecklistSection）：
  - 查询直接子任务：`taskQuery({ parentId: task.id })`。
  - **添加**：输入框回车（`taskCreate({ title, parentId: task.id })`，`listId` 继承父任务）。
  - **行内操作**：勾选完成/取消完成（触发聚合）、行内改名（复用 `useTaskRename`）、上移/下移（`taskReorderSubtasks`）、两步删除（有子任务 → 处置确认框）。
  - **点击行 → 打开该子任务详情**（`onOpenTask(childId)`，由 TasksPage 注入 `setSelectedId`）。
  - 冻结规则：与检查项一致——任务 completed/archived 时只读。
  - 进度：`x/y` 徽标。
- 详情面板删除按钮：有子任务 → 处置确认框（同 7.2）。

### 7.4 设置页（`src/features/settings/SettingsPage.tsx`）

- 新增「子任务」区块（放在任务相关区域），两个开关：
  - 「子任务全部完成时自动完成父任务」（`subtaskAutoCompleteParent`）
  - 「完成父任务时级联完成子任务」（`subtaskCascadeChildren`）
- 沿用现有 settings 保存模式（`settingsSave`）。

### 7.5 其他视图

- 今日/搜索/智能列表/托盘/CLI/CSV：**零改动**（子任务是完整任务，天然参与）。CSV 导入导出不新增 parent 列（范围外，仅保证子任务作为独立行参与）。

## 8. 测试计划

### 8.1 后端（`src-tauri/src/application/tasks.rs` 内新增测试，沿用 `open_service()` / `test_db()` 模式）

1. **创建与查询**：带 parent 创建、继承 list_id、child_order 追加、`query_tasks({parent_id})` 过滤。
2. **树形闭包**：`query_tree` 包含祖先 + 后代、去重、软删排除、多根。
3. **循环防护**：`set_task_parent` 拒绝「自己 / 后代 / 祖先链环」。
4. **深度上限**：第 6 层拒绝（create 与 set_parent 两路）。
5. **完成聚合（默认开关）**：
   - 所有子任务完成 → 父自动完成（含多级向上）；
   - 父完成 → 级联子任务完成；
   - 子取消完成 → 父恢复 todo；父取消完成 → 子恢复 todo（双向）。
6. **开关关闭**：`subtask_auto_complete_parent=false` → 子全完成父不动；`subtask_cascade_children=false` → 父完成子不动。
7. **归档级联**：archive/unarchive 目标 + 后代。
8. **删除处置**：Cascade 删除后代且无孤儿（检查项、展开状态）；Promote 直接子任务提升、孙代保留、list_id 不变。
9. **归巢**：`set_task_parent` 后 child_order 追加、list_id 不变；提升后 sort_order 重排。
10. **展开状态**：set/清除/随删除清理。
11. Schema 断言测试更新（db/mod.rs 三处 21 → 22）。

### 8.2 前端

- `pnpm typecheck`、`pnpm lint`（若配置）。
- 手动/后续单测：树形组装（parentId→children、排序、展开过滤）抽为纯函数并单测。

## 9. 兼容性与风险

- **旧数据**：`parent_id`/`child_order` 均为 NULL/0，顶层任务不受影响；`task_tree_expanded` 新表无历史数据（默认全展开）。
- **reorder_tasks（旧）**：仍用于顶层/智能列表排序；树内兄弟排序走 `reorder_subtasks`。旧 `taskReorder` 调用方（TasksPage 顶层拖拽）改为在树形上下文中调用 `taskReorderSubtasks(null, ids)`（顶层兄弟）。
- **搜索索引**：任务 title/notes 已建索引；子任务作为独立任务自动可搜。级联完成不改标题，无需重索引。
- **风险点**：树形拖拽（dnd-kit 嵌套）复杂度最高 → 实现时优先保证「间隙重排」正确，再补「归巢区」；递归完成聚合需事务内一次性处理，避免 N+1 与重复 spawn。
- **自动化**：级联改动不触发规则（注记，范围外）。

## 10. 文档更新

- `docs/development-roadmap.md`：将「多层子任务」从「暂不包含」说明中移除/标注已完成。
- 可选：`docs/post-v1-iteration-design.md` 或 release notes 补一段子任务说明。
