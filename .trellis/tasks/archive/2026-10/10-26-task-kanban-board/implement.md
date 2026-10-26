# 实施计划：任务看板视图

> 依 `design.md` 实施。顺序：分支 → 迁移 → 领域 → 服务 → IPC → 前端绑定 → BoardPage → 路由/侧边栏 → 验证收尾 → 文档。
> 精确挂点：`design.md` §2/§4/§7 所引 file:line。

## 0. 前置

- [x] `git checkout -b feat/task-kanban-board`（基于最新 main；main 有 PR 保护，禁止直推）
- [x] 阅读 `.trellis/spec/tauri/index.md`（nullable 列读法、schema 断言、三态更新语义坑位）与 `.trellis/spec/frontend/state-management.md`（chips 持久化、URL 单向同步、queryKey 纪律）

## 1. 数据库迁移

- [x] 新建 `src-tauri/migrations/0024_task_board_stage.sql`（ALTER TABLE tasks ADD COLUMN board_stage TEXT）
- [x] `infrastructure/db/mod.rs`：MIGRATIONS 追加 24；三处 schema_version 断言 23 → 24
- [x] 验证：`cd src-tauri && cargo test`（迁移健康/幂等/备份用例过）

## 2. 领域模型（domain/task.rs）

- [x] `TaskBoardStage`（仅 `Doing`，serde camelCase，as_str/parse）
- [x] `Task.board_stage: Option<TaskBoardStage>`；`tasks.rs` SELECT 列清单（:2504-2506）与 `map_row` 同步加 `t.board_stage`（**风险 1，漏改即错位**）

## 3. 服务层（application/tasks.rs）

- [x] `set_task_board_stage(id, Option<TaskBoardStage>)`：todo 校验、置位/清空、revision+1
- [x] `complete_task` / `archive_task` 事务内置空 `board_stage`（不变式：stage 只在 todo 有意义）
- [x] 单测（design §9 后端 4 项），`cargo test` 全绿

## 4. IPC 命令层

- [x] `commands/mod.rs` 新增 `task_set_board_stage`（stage 为 `Option<String>`，parse 校验）+ emit `domain://changed`
- [x] `lib.rs` invoke_handler 注册；`cargo check` 过

## 5. 前端 IPC 绑定（src/ipc/client.ts）

- [x] `Task.boardStage: "doing" | null`；`taskSetBoardStage(id, stage)`

## 6. BoardPage（src/features/board/）

- [x] `splitBoardColumns` 纯函数 + vitest（todo 分流 / completed 列 / chips 透传）
- [x] `BoardPage.tsx`：三列布局、列头徽标、双查询 + overview chips（`board.groupFilter` 持久化、`?group=` 单向同步）
- [x] `BoardCard.tsx`：精简卡片；点击详情面板、右键复用现有菜单
- [x] DnD：单 DndContext + 列 droppable；onDragEnd 三向分发（stage 置位/清空、complete/uncomplete 含撤销 toast）；乐观更新 + invalidate

## 7. 路由与侧边栏

- [x] `AppRouter.tsx` 加 `/board`；`MainShell.tsx` 导航加「看板」项

## 8. 验证（review gate）

- [x] `cd src-tauri && cargo test` 全绿（含 schema 断言 24；最终门禁 197 lib + 13 ai_eval）
- [x] `pnpm typecheck`、`pnpm test:unit` 全绿（12 文件 / 59 测试，含 splitBoardColumns 6 项）
- [x] 对照 prd.md Acceptance Criteria 逐条核对（2026-10-26：独立审查代理逐维度核对过——列映射、不变式路径、拖拽语义、chips 同步、复用改造零行为变更均「过」；三条发现已修复并加回归测试。纯 GUI 手感项（拖拽视觉、toast、真实窗口）留合并后人工过——本环境不运行 GUI）
- [x] `python3 ./.trellis/scripts/task.py validate 10-26-task-kanban-board`

## 9. 文档与收尾

- [x] `docs/ui-layout-interaction.md`：§5.1 六入口、§5.2 `/board` 路由、新增 §5.4 看板小节、§16 标注已实现（development-roadmap.md 无看板条目，无需标注）
- [x] spec 回写：tauri schema 断言修正为「五处」（db/mod.rs ×3 + backup.rs ×2，本次 restore 测试抓出的真实缺口）；frontend state-management 新增「乐观补丁判定 ≠ 渲染分流」坑 → 提交 → PR

## 回滚点

- 每个提交独立可 revert；迁移 0024 为单列纯增量，revert 代码后旧库可继续打开。
