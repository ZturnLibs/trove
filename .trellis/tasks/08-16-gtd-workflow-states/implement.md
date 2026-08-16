# Implement: GTD 工作流

> **纪律**：每 Phase 结束跑 quality-bar 对照；不通过不进入下一 Phase。  
> 验证命令：`cargo test`、`pnpm typecheck`、`pnpm test:unit`、`pnpm build`

---

## Phase 0 — 契约与迁移（无 UI）

- [x] 新增 `src-tauri/migrations/0010_gtd_workflow.sql`
- [x] 新增 `domain/task_activity.rs` + 全组合单测
- [x] 扩展 `Task` / `TaskQuery` / TS 类型
- [x] `query_tasks` 接入 active 谓词；`search` 路径豁免 availableAt 过滤
- [x] `today_tasks` / `counts` 接入 active 谓词
- [x] **门禁**：migration 测试 + activity 单测通过（`cargo test` 60/60）

---

## Phase 1 — 推迟显示 + 已推迟视图

- [x] `task_set_defer` 命令 + 冲突校验（due vs availableAt）
- [x] `DeferPicker` 组件 + TaskDetailPanel 分区 UI
- [x] TaskRow 右键推迟预设 + recent-actions
- [x] TasksPage chip「已推迟」+ 空状态文案
- [x] coach mark（推迟 ≠ 延期）
- [ ] **极致门禁**：
  - [ ] S2 场景 ≤3 键完成推迟（待手动验收）
  - [x] 搜索推迟任务 1 步找到（deferredOnly + search 并存）
  - [x] 撤销推迟可用

---

## Phase 2 — 等待事项

- [x] `task_set_waiting` / `task_clear_waiting`
- [x] TaskDetailPanel 等待表单
- [x] Today「等待跟进」区 + 行内动作
- [x] 标记等待时自动移出今日重点 + toast（daily_focus 清除；recent-actions 撤销）
- [ ] **极致门禁**：
  - [ ] S3 场景单屏完成（待手动验收）
  - [x] follow-up 到期出现在正确区，不污染 due_today

---

## Phase 3 — 今日重点

- [x] `daily_focus_*` 命令全套
- [x] Today focus 区 UI + 拖拽排序（复用 @dnd-kit）
- [x] 键盘 `F` / `Shift+F`
- [x] 昨日结转横幅 + carry 命令
- [x] 超过 5 项温和提示
- [ ] **极致门禁**：
  - [ ] S1 场景 ≤2 键加入重点（待手动验收）
  - [ ] 拖入拖出 focus 区无布局跳动（待手动验收）
  - [ ] 跨日启动结转流程顺畅（待手动验收）

---

## Phase 4 — 整合与文档

- [x] 更新 `docs/keyboard-shortcuts.md`（Today 页新键位）
- [x] 更新 `docs/ui-layout-interaction.md` Today 区块说明
- [x] SmartList `Deferred` / `WaitingFollowUp` + Tasks 筛选与 SavedView 兼容
- [x] 全链路验证：`cargo test` 66/66 · `pnpm typecheck` · `pnpm test:unit` 35/35 · `pnpm build`
- [ ] **归档门禁**：父级 `quality-bar.md` 九维 checklist（待手动极致验收 S1–S4）

---

## 建议 PR 拆分

| PR | 内容 |
| --- | --- |
| PR1 | Phase 0 + Phase 1 |
| PR2 | Phase 2 |
| PR3 | Phase 3 + Phase 4 |

每 PR 独立可测，避免 big-bang。

---

## Rollback

- migration 0010 仅 ADD COLUMN / 新表，可写 down migration 删除列（开发环境）
- 功能开关不需要；新字段默认值保证旧 UI 路径仍可用至 Phase 1 发布
