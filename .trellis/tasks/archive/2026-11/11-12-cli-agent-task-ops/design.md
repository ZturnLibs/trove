# Design:trove-cli 完整管理能力

## 架构与边界

保持既有三层不动,全部为**增量**:

```
trove-cli (bin/trove-cli.rs)          只加 CLI 参数解析、清单名解析、批量循环、输出格式化
   │  trove-action:{json} argv 转发(协议不变)
   ▼
WorkbenchAction / ActionOutcome       增量加变体(domain/workbench_action.rs)
   │  dispatch(...)
   ▼
application/workbench_actions.rs      加 match 臂,调既有 TaskService 方法
   ▼
TaskService / ListService             零改动(应用层 API 已齐备,见 prd.md Background)
```

- 协议不变:`trove-action:` 前缀 + 单实例转发 + 临时文件回写(`cli_protocol::handle_cli_dispatch`,`src-tauri/src/lib.rs:510`)。
- 前端/UI 零影响:新变体仅 CLI / `Raw` 入口可达;`workbench_action_dispatch` IPC 命令签名不变。

## 新增 WorkbenchAction 变体

serde tag `action` / camelCase(workbench-action.md 契约):

| 变体 | 字段 | 读/写 | 落到 TaskService |
| --- | --- | --- | --- |
| `QueryTasks` | `query: TaskQuery`(直接复用 domain 结构,`#[serde(default)]`) | 读 | `query_tasks` → `TasksQueried { items: Vec<ActionTaskHit>(task_hit 映射), total }` |
| `GetTask` | `task_id: EntityId` | 读 | `get_task` → `TaskFetched { task: Task }` |
| `UpdateTask` | `input: ActionUpdateTaskInput, confirmed: bool` | 写 | `get_task` → 合并 → `update_task` → `TaskUpdated { task: Task }` |
| `ReopenTask` | `task_id, confirmed` | 写 | `uncomplete_task` → `TaskReopened { task: Task }` |
| `ArchiveTask` | `task_id, confirmed` | 写 | `archive_task` → `TaskArchived { task: Task }` |
| `DeleteTask` | `task_id, disposition: TaskDeleteDisposition, confirmed` | 写 | `delete_task_tree` → `TaskDeleted { task_id }`。CLI `--children cascade|promote`（应用层枚举仅此两种，无 archive 处置） |
| `PostponeTask` | `task_id, days: i64, confirmed` | 写 | `postpone_task` → `TaskPostponed { task: Task }` |
| `SetTaskDefer` | `task_id, available_at: Option<String>, confirmed` | 写 | `set_task_defer`(Some=设置,None=清除)→ `TaskDeferred { task: Task }` |
| `QueryLists` | 无 | 读 | `list_lists` + 逐清单 `count_list_todo_tasks` → `ListsQueried { lists: Vec<ActionListHit> }` |

`ActionUpdateTaskInput`(部分字段语义;`UpdateTaskInput` 要求完整字段的应用层契约不变):

```rust
pub struct ActionUpdateTaskInput {
    pub id: EntityId,
    pub title: Option<String>,        // Some=改
    pub notes: Option<String>,        // Some=改(空串=清空)
    pub priority: Option<TaskPriority>,
    pub list_id: Option<EntityId>,
    pub due_date: Option<String>,     // 与 clear_due 配对
    pub due_time: Option<String>,
    pub clear_due: bool,              // true=清空 due_date+due_time,优先于字段
    pub tag_names: Option<Vec<String>>, // None=不改,Some(vec)=整体替换(空 vec=清空)
}
```

> 规范坑则落地:`Option` 无法区分「不改」与「清空」,凡可清空的可选字段必须配显式 bool(spec/tauri/index.md「Clearing an optional relation」)。

同步扩展 `requires_confirmation()` / `is_confirmed()` 的 match 臂:UpdateTask / ReopenTask / ArchiveTask / DeleteTask / PostponeTask / SetTaskDefer。

## CLI 子命令映射

```
query tasks  [--status todo|completed|archived] [--list <名|uuid>] [--search S]
             [--priority L] [--due-from D] [--due-to D] [--parent ID]
             [--limit N] [--offset N]
query lists
show         <task-id>
create task  --title T [--notes N] [--due-date D] [--due-time T] [--list <名|uuid>]
             [--priority P] [--tag NAME]... [--parent ID] [--yes]
create-batch [--file f.ndjson | stdin] [--list <名|uuid>] [--yes]   # 上限 100
update       <id> [--title T] [--notes N] [--due-date D] [--due-time T]
             [--clear-due] [--priority P] [--list <名|uuid>] [--tag N]... [--yes]
reopen       <id> [--yes]
archive      <id> [--yes]
delete       <id> --children delete|archive|promote [--yes]
postpone     <id> --days N [--yes]
defer        <id> [--at DATE | --clear] [--yes]
```

- 既有 `today / inbox / search / complete / query today|overdue|inbox|memories|snippets / export-csv / raw / --dry-run / --json / --app` 全部不动。
- `create` 新 flags 映射到既有 `CreateTaskInput` 字段(`--tag` 重复参数收集为 `Vec<String>`);`--list` 按名称解析。

## 清单名解析(CLI 侧,D3)

1. CLI 先转发 `QueryLists`;
2. 输入匹配规则:与 uuid 相等 → 直接用;否则按 name **精确匹配**(区分大小写);
3. 命中 0 个 → 报错并列出全部清单名;命中 >1 个 → 报错并附候选(理论上清单名可重复,不静默取第一个);
4. `create-batch` 的全局 `--list` 解析一次,复用到每条;行内 `list` 字段优先于全局。

## 批量导入(CLI 侧循环,D2)

- 输入:NDJSON,每行一个对象 `{ "title": …, "notes": …, "dueDate": "YYYY-MM-DD", "dueTime": "HH:MM", "priority": "high", "list": "名称", "tags": ["…"], "parent": "uuid" }`;字段与单条 `create` 对齐(camelCase,与动作 wire 一致)。
- 逐条构造 `CreateTask { input, confirmed: true }` 转发;任一条失败记录 `{ index, error }` 继续;结尾输出 JSON 数组 `[{ index, taskId?, title, ok, error? }]`,人类可读模式打印汇总。
- 上限 100 条(超过直接拒绝,防 Agent 失控循环);空标题行跳过并计为失败项。
- 进程级失败(应用未启动且启动失败、转发超时)时终止循环并返回已完成部分。

## dispatch 实现要点

- 新 match 臂统一走 `state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?`(与 QueryToday 一致,workbench_actions.rs:91)。
- `UpdateTask` 合并逻辑:`get_task(id)?` → 每个 `Option` 字段 None 保旧值 → 组装 `UpdateTaskInput` → `update_task`。整个合并在应用进程内一次 dispatch 完成,无跨进程读-改-写竞态。
- 测试性:dispatch 臂体薄(取 service → 调一个 `pub(crate) fn`);`pub(crate) fn` 接 `&TaskService` 参数,可像 `application/tasks.rs` 现有测试一样用 tempdir `Database` 直测(注意 tempdir 生命周期坑则)。
- `ActionListHit { id, name, kind, open_count }`:不暴露 group 结构,Agent 只需要平铺清单表(收件箱含内)。

## 兼容与回滚

- 纯增量:不改既有变体字段、不改 outcome 既有形态、不改 IPC/协议;回滚 = 还原三个文件的增量 + 文档。
- `Raw` 子命令继续可用;新变体自动可被 `Raw` 派发(wire 示例写进 docs/cli.md 附录)。
- 安装包契约不变:`default-run` / lipo / `beforeBundleCommand` 均不触碰(extra-binaries-and-release.md 陷阱规避)。

## 文档与 Skill(R8/R9)

- `docs/cli.md`:重写——全局选项、全命令表、Agent 使用章节(解析 `--json` outcome、`--yes` 语义、应用自启动行为)、钉钉→批量建任务可复制示例、`Raw` wire 附录。
- `docs/action-layer.md`:动作表补 9 个新变体行。
- `.opencode/skills/trove-cli/SKILL.md`:front-matter(name/description)+ 命令速查 + 三条 recipe(钉钉聊天→批量任务、逾期巡检、修正/回收任务)+ 安全约定(先 `--dry-run`、写操作必 `--yes`、`TROVE_APP` 指向 dev 实例隔离测试)。

## 风险

| 风险 | 缓解 |
| --- | --- |
| Agent 误删/误改真实数据 | 写操作一律 `--yes` 显式门;`delete` 强制 `--children`;skill 教程先 `--dry-run` |
| 清单重名静默错插 | D3 精确匹配 + 歧义报错列候选 |
| 批量循环部分失败后 Agent 误判 | 输出逐条 ok/error 数组,人类模式打汇总 |
| `TaskDeleteDisposition` 若未派生 serde | 实现时确认;未派生则补 `#[derive(Serialize, Deserialize)]`(纯加法) |
