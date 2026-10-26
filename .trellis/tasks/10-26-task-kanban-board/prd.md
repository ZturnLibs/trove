# 任务看板视图

## Goal

为 Trove 增加独立看板页（`/board`）：待办 / 进行中 / 已完成三列，拖拽推进任务阶段，作为列表 / 树之外的第三种任务空间形态。解决列表视图缺乏「推进感」、多任务并行时状态一眼不可见的问题。

## Background（codebase facts）

- 路线图定位：看板列在 v1.2 后备视图清单（`docs/ui-layout-interaction.md:621`）。
- `TaskStatus` 仅 `todo/completed/archived`，「进行中」维度缺失（`domain/task.rs:9-12`）——本任务补齐。
- 统一活跃定义：默认 `TaskQuery{status: todo}` 自动排除 waiting 与未到期 deferred（`application/tasks.rs:1301-1308`），看板直接复用，零新增过滤逻辑。
- 完成/撤销语义由 `complete_task`（`tasks.rs:447`，含 recurring 生成下一期）与 `uncomplete_task`（`:671`）承载，看板拖拽必须走这两个入口。
- 分组过滤、`task_list_overview`、分组 chips 交互模式（今日页 `today.groupFilter`）刚在 09-08-task-list-groups 落地，直接复用。
- dnd-kit 已有三套实战（`TaskSidebarTree` 嵌套 droppable、三个页面行排序，PointerSensor + distance 防误拖）。

## Requirements

### R1 列语义 = 阶段列（用户确认 2026-10-26）

- 迁移 0024（纯增量）：`tasks` 加可空 `board_stage`（唯一合法值 `'doing'`，NULL = 待办）。
- 三列：**待办**（`status=todo` 且 `board_stage IS NULL`）、**进行中**（`status=todo` 且 `board_stage='doing'`）、**已完成**（映射现有 `status=completed`，不新增字段）。
- 不变式：`board_stage` 只在 `status=todo` 时有意义；`complete_task` 完成任务时置空 `board_stage`。

### R2 内容范围（用户确认 2026-10-26）

- 全局看板：默认展示全部可推进任务（跨清单/分组，遵循统一活跃定义——waiting/deferred 不上板）。
- 顶部分组过滤 chips：`全部 | 各分组 | 未分组`，复用今日页 chips 交互；选择 localStorage 持久化（key `board.groupFilter`），默认全部。
- 已完成列边界 = **今日完成**（`completed_since=今天`），有界不膨胀。

### R3 入口与路由（用户确认 2026-10-26）

- 侧边栏新增「看板」顶级导航项；路由 `/board`，支持 `?group=` 深链。
- 与任务页完全解耦，不改 TasksPage。

### R4 拖拽与卡片交互

- 拖拽语义：待办↔进行中 = `board_stage` 置位/清空；拖入完成列 = 走 `complete_task`（recurring 按既有语义生成下一期、支持撤销 toast）；拖出完成列 = 走 `uncomplete_task` 并回待办（stage 复位为空）。
- 卡片点击 → 打开现有详情面板；右键 → 复用现有任务右键菜单。拖拽只改 stage/status，不改层级与清单归属。
- **列内新建**（2026-10-26 追加）：待办/进行中列提供「＋ 新建任务」行内 composer（Enter 创建、Esc 关闭、成功后保持开启便于连续添加）；清单默认：分组 chips 下取该组首个清单（新任务在当前过滤下可见），全部/未分组落收件箱；进行中列创建后立即置 doing。已完成列不提供新建。
- 列内排序继承现有排序（清单内 `sort_order` → 到期日），v1 不做列内手动排序。

## Acceptance Criteria

- [ ] 侧边栏出现「看板」项，`/board` 直达；默认「全部」chips，三列渲染待办/进行中/已完成（今日），列头带计数徽标
- [ ] 分组 chips 切换后三列同步过滤；选择跨会话记忆
- [ ] 卡片拖入「进行中」/拖回「待办」，刷新后状态保持（`board_stage` 持久化）
- [ ] 卡片拖入「已完成」：任务完成（`completed_at` 写入、可撤销 toast）；recurring 任务完成后下一期出现在待办列
- [ ] 已完成卡片拖回：任务回到待办列（`board_stage` 为空、`status=todo`）
- [ ] waiting/deferred 任务不出现在任何列；已推迟任务到预期日期后自然上板
- [ ] 待办/进行中列可快速新建：全部 chips 下新任务落收件箱出现在待办列；分组 chips 下落该组清单；进行中列新建直接以进行中状态出现
- [ ] 存量数据升级零感知（迁移 0024 后全部任务 `board_stage=NULL`，行为不变）
- [ ] `cargo test`（含 schema 断言 24）、`pnpm typecheck`、`pnpm test:unit` 全绿

## Out of Scope

- 甘特、月历（路线图另列）
- 列内手动排序、自定义列/多看板、列 WIP 上限
- 多选批量拖拽；waiting/deferred 任务的看板呈现与看板开关
- 托盘/快速捕获联动看板

## Technical Notes

- 查询零新增：`task_query({status: todo, listGroupId})` 客户端按 `boardStage` 分流两列；`task_query({status: completed, completedSince: today, listGroupId})` 为完成列。仅需新增 stage 写路径 IPC。
- 无需新索引：任务量为个人工作台量级，过滤走既有 status 条件后内存分流。
