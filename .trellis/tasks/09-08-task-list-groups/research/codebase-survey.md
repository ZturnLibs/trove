# 调研：任务分组（分组→清单两级）代码底座

> 2026-09-08 主会话调研结论。v2.1.0 代码基线。

## 结论先行

「个人 vs 工作」分组所需的数据机制大部分已存在：`task_lists` 清单表（每任务必属一个清单）+ 标签 + 智能列表 + 保存视图。**缺的不是数据模型，而是：① 清单之上没有「分组」层级；② 清单入口藏在任务页下拉框，侧边栏不可见。** 本次新增 `task_list_groups` 实体 + 侧边栏两级导航。

实测用户数据库（2026-09-08）：只有收件箱一个清单，78 个任务全在其中（18 todo / 60 completed），标签数为 0——清单/标签机制从未被使用，佐证「入口太深」是主要问题。

## 应用架构

- Tauri 2 + React 19 + TypeScript + SQLite（rusqlite, WAL, foreign_keys ON）。中文 UI。
- 分层：`domain/` → `application/` 服务 → `commands/` Tauri IPC → 前端 `src/ipc/client.ts` 绑定。
- DB 文件：`app_data_dir(com.trove.app)/workbench.db`；迁移失败自动做 pre-migrate 快照备份。

## 关键挂点（file:line，基于 v2.1.0）

### 后端

| 挂点 | 位置 |
|---|---|
| Task/TaskList/ListKind/Tag/TaskQuery/SmartListKind 领域类型 | `src-tauri/src/domain/task.rs:66-260`（TaskList 92-102，TaskQuery 213-237） |
| 清单 CRUD（含删除三选一 disposition + 撤销）| `src-tauri/src/application/tasks.rs:70-289` |
| 种子收件箱（首次启动建 `收件箱` kind=inbox）| `src-tauri/src/application/tasks.rs:28-50` |
| **`build_task_filters()` —— 唯一 WHERE 构造器**，分组过滤加这里 | `src-tauri/src/application/tasks.rs:1203-1296` |
| `query_tasks`（ORDER BY sort_order）/ `query_tree` / `smart_list` / `today_tasks` / `counts` | `tasks.rs:1318/1081/1363/1797/1963`，TaskCounts 定义 `tasks.rs:2390` |
| TASK_ROW_SELECT + map_task_row（列顺序敏感）| `application/tasks.rs` |
| 命令注册 | `src-tauri/src/lib.rs:591-754`（invoke_handler），`run()` 在 lib.rs:442 |
| 迁移数组 + schema 断言（三处，当前 22）| `src-tauri/src/infrastructure/db/mod.rs:6-47, 217, 229` |
| 迁移文件目录 | `src-tauri/migrations/0001..0022`，下一个 **0023** |
| 保存视图（filter_json 不透明 JSON）| `application/saved_views.rs:7-16`，迁移 0009 |
| 侧边栏计数 IPC `task_counts()`（前端 15s refetch）| `commands/mod.rs`；`TaskCounts` tasks.rs:2390 |

### 前端

| 挂点 | 位置 |
|---|---|
| 侧边栏固定项：今日/收件箱/任务/记忆/剪切板/设置 + 徽标 | `src/app/layouts/MainShell.tsx:25-31, 120-163` |
| 路由（`/tasks/:listId` 已存在但 TasksPage 忽略参数）| `src/app/router/AppRouter.tsx:64-76`；TasksPage.tsx:71 |
| 任务页筛选下拉（smart/search/list/status/priority/tag/保存视图）| `src/features/tasks/TasksPage.tsx:53-63, 618-822` |
| 任务页视图状态持久化形状 `{listId, status, priority, tagId, smart, showDeferred, showWaiting}` | `TasksPage.tsx:120-131` |
| 清单管理右键菜单（任务页内）| `TasksPage.tsx:1022-1043` |
| `TaskGroup` 粘性分组头（可复用做按清单分区）| `src/features/tasks/TaskLayout.tsx:53-86` |
| 今日页五区块（重点/等待/逾期/提醒/今日任务/已完成）| `src/features/today/TodayPage.tsx:1191-1363` |
| 详情面板清单切换器 | `src/design-system/patterns/TaskDetailPanel.tsx:278-284` |
| 快速捕获窗口（capture/search/clip 三模式 + NL 解析日期/优先级/重复）| `src/features/search/QuickWindow.tsx:144-148, 442-495` |
| 任务树组装纯函数 buildForest/flattenVisible | `src/features/tasks/taskTree.ts:36-120`（有单测 taskTree.test.ts） |
| dnd-kit 已在依赖中 | `package.json` |

## 模式与约定（写代码前必读）

- `.trellis/spec/tauri/index.md`：cargo test --lib 门槛；**schema 断言必须等于最新迁移号**；nullable 列必须 `Option<String>` 读；tempdir 生命周期；partial settings 结构体也要 camelCase。
- 迁移步骤：新建 `0023_*.sql` → MIGRATIONS 注册 → db/mod.rs 三处断言 22→23。
- serde 一律 `#[serde(rename_all = "camelCase")]`；TS 侧镜像类型在 `src/ipc/client.ts`。
- 保存视图 filter_json 是不透明 JSON，加字段向后兼容，无需迁移。
- 提交风格：中文 `feat:` / `fix:` 前缀。

## 主流应用参考（方案讨论结论）

- Apple 提醒事项/滴答清单：单层清单；Things：区域→项目两级；Todoist：项目→分段；TickTick：文件夹→清单。
- 共识：清单/分组是侧边栏一等公民；今日等智能视图跨清单聚合（本次以过滤 chips 呈现）。

## 用户已确认的产品决策（2026-09-08 会话）

1. 两级结构：分组（工作/个人）→ 清单（组内可多个）。
2. 分组全生命周期用户管理：新建/重命名/拖拽排序/删除（删除=解散：清单回未分组，任务不动，可撤销）。
3. 收件箱固定顶部，不可归组。
4. 今日页用过滤 chips（全部/各分组），五区块统一过滤；托盘今日弹窗 v1 保持全部。
5. 快速捕获默认收件箱 + 目标清单选择器。
6. v1 不做颜色/图标；只动任务模块（记忆/剪切板不动）。
