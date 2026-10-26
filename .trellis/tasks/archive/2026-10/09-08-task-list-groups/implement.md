# 实施计划：任务分组（分组 → 清单两级结构）

> 依 `design.md` 实施。顺序：分支 → 迁移/数据层 → 领域/服务层 → IPC → 前端绑定 → 侧边栏 → 任务页 → 今日页 → 快速捕获 → 验证收尾 → 文档。
> 精确挂点见 `research/codebase-survey.md`。

## 0. 前置

- [x] `git checkout -b feat/task-list-groups`（基于最新 main；main 已开启 PR 保护，禁止直推）
- [x] 阅读 `.trellis/spec/tauri/index.md`（Quality Check + Pitfalls：nullable 列读法、schema 断言、tempdir）

## 1. 数据库迁移

- [x] 新建 `src-tauri/migrations/0023_task_list_groups.sql`（design §2 的建表 + ALTER + 索引）
- [x] `infrastructure/db/mod.rs`：MIGRATIONS 追加 23；三处 schema_version 断言 22 → 23
- [x] 验证：`cd src-tauri && cargo test`（迁移健康/幂等/备份用例过）

## 2. 领域模型（domain/task.rs）

- [x] `TaskListGroup`、`TaskListGroupOverview` / `TaskListGroupNode` / `TaskListSummary`（serde camelCase）
- [x] `TaskList` 加 `group_id: Option<EntityId>`（含既有 map 行）
- [x] `TaskQuery` 加 `list_group_id: Option<EntityId>`
- [x] 清单创建/更新输入扩展归组字段（按 design §3 的 clear_group 语义决策）
- [x] `ListGroupDeleteUndo` 类型

## 3. 服务层（新文件 application/list_groups.rs，impl TaskService 扩展）

- [x] `list_group_create / rename / reorder`（重名校验、sort_order 追加/重写、revision+1）
- [x] `list_group_delete`：事务内解散（清单 group_id=NULL）+ 软删，返回 undo 数据；注册撤销（对齐 tasks.rs:70-289 清单删除撤销机制）
- [x] `update_task_list` 归组/移出 + inbox 拒绝归组校验
- [x] `task_list_overview()` 聚合（design §4.3）
- [x] `build_task_filters` 接入 `list_group_id`（tasks.rs:1203）
- [x] `today_tasks` 加可选 listGroupId 参数
- [x] 单测（design §7.1 全部 7 项），`cargo test` 全绿

## 4. IPC 命令层

- [x] `commands/mod.rs` 新增 5 个命令 + 扩展 3 个（design §5 表）；事件 emit 对齐现有清单命令
- [x] `lib.rs` invoke_handler 注册；`cargo check` 过

## 5. 前端 IPC 绑定（src/ipc/client.ts）

- [x] TS 类型镜像 + 5 个新方法；`TaskList.groupId`、`TaskQuery.listGroupId`、`todayTasks` 参数

## 6. 侧边栏（MainShell.tsx）

- [x] 任务区数据驱动（taskListOverview + 15s refetch）
- [x] 分组行（折叠 localStorage + 徽标 + 点击过滤导航）与清单行（`/tasks/:listId` 导航）
- [x] dnd-kit：清单拖入/拖出分组、分组排序；收件箱不参与
- [x] 右键菜单全套（design §6.2）+ 行内重命名 + 删除撤销 toast
- [x] 新建分组/清单入口

## 7. 任务页（AppRouter.tsx / TasksPage.tsx / TaskDetailPanel.tsx）

- [x] `/tasks/:listId` 参数生效 + `/tasks/group/:groupId` 新路由；URL↔状态防循环同步
- [x] 清单筛选下拉 optgroup 分层 + 分组过滤
- [x] 「按清单分区显示」开关（复用 TaskGroup 粘性分节）
- [x] 「移动到清单」菜单分层（详情面板 + 右键）
- [x] 一次性空状态引导（一键创建 工作/个人 分组骨架）

## 8. 今日页（TodayPage.tsx）

- [x] 过滤 chips（全部/各分组/未分组）→ `todayTasks({ listGroupId })`
- [x] 选择 localStorage 持久化；托盘不动

## 9. 快速捕获（QuickWindow.tsx）

- [x] 任务捕获模式加清单选择器（默认收件箱，按分组分层）

## 10. 验证（review gate）

- [x] `cd src-tauri && cargo test` 全绿（含 schema 断言 23）
- [x] `pnpm typecheck`、`pnpm test:unit` 全绿
- [x] 对照 prd.md Acceptance Criteria 逐条核对；手工矩阵过（老库升级、拖拽、撤销、直达路由）（2026-10-26 复核：10 条 AC 锚点全部核实，cargo test 193+13 / typecheck / test:unit 53 全绿。自动化已覆盖：老库升级=迁移健康/幂等/备份用例，撤销=解散/undo 单测，直达路由=AppRouter 路由+TasksPage 单向同步断言；纯 GUI 手感项（拖拽视觉、toast、真实窗口跳转）留待人工 review——本环境不运行 GUI）
- [x] `python3 ./.trellis/scripts/task.py validate 09-08-task-list-groups`

## 11. 文档与收尾

- [x] `docs/ui-layout-interaction.md` 侧边栏小节；`docs/development-roadmap.md` 状态标注
- [x] Phase 3：spec 更新（新模式/坑回写 `.trellis/spec/`）→ 提交计划确认 → 提交 → PR（`task.py create-pr`）（2026-10-26 执行：tauri spec 补「清空可选关系需显式 clear 标志」坑位；`frontend/state-management.md` 从空模板落成实际约定——视图偏好 localStorage 判据、URL↔状态单向同步、queryKey 纪律；feat/docs/spec 三笔提交在案，随后 `task.py create-pr` 建 PR 并回写链接）

## 回滚点

- 每个提交独立可 revert；迁移 0023 为纯增量（新表 + 可空列），revert 代码后旧库可继续打开（多出的表/列不影响旧代码显式列名读写——已确认：全部 `INSERT INTO task_lists`（tasks.rs、reminders.rs、backup.rs）均为显式列清单）。
