# v2.0 任务检查项数据模型（切片 6）

> 父任务：`08-19-v2-0-ai-assist-roadmap`（任务地图 6：非 AI 前置，为切片 7 任务拆分铺路）。
> §9.3 依据：「任务拆分——默认生成**一层检查项**，不自动创建多个带提醒的任务」；本切片先交付检查项本体。

## Goal

任务支持一层检查项（checklist）：详情面板可增删改/勾选，任务行显示 `2/5` 进度；完成全部检查项**不**自动完成任务（用户决定）；v1.x 行为零变化。数据与命令就绪后，切片 7 的 AI 拆分只是"往这个模型里填候选"。

## Requirements

1. 迁移 `0020_task_checklist_items.sql`：
   ```sql
   CREATE TABLE task_checklist_items (
     id TEXT PRIMARY KEY NOT NULL,
     task_id TEXT NOT NULL REFERENCES tasks(id),
     content TEXT NOT NULL,
     checked INTEGER NOT NULL DEFAULT 0,
     sort_order INTEGER NOT NULL DEFAULT 0,
     created_at TEXT NOT NULL,
     updated_at TEXT NOT NULL,
     revision INTEGER NOT NULL DEFAULT 1,
     deleted_at TEXT
   );
   CREATE INDEX idx_checklist_task ON task_checklist_items(task_id, deleted_at);
   ```
   软删沿用项目惯例；sort_order 按任务内连续整数。
2. 领域类型（domain/task.rs）：`ChecklistItem { id, taskId, content, checked, sortOrder, createdAt, updatedAt, revision }` + `TaskChecklist { items, total, checkedCount }`。
3. 服务（TaskService，就近内聚）：
   - `checklist_list(task_id)`：按 sort_order；
   - `checklist_add(task_id, content)`：trim 非空、上限 50 项（防膨胀）、追加尾部；
   - `checklist_update(id, { content?, checked? })`：软删/完成态不可改（任务 completed 冻结）；
   - `checklist_delete(id)`：软删 + **normalize**（重排剩余连续序）；
   - `checklist_reorder(task_id, ordered_ids)`：与任务拖拽同语义；
   - 全部操作 `UPDATE tasks SET updated_at` 触碰父任务（列表排序新鲜度）+ 不 bump tasks.revision（检查项非任务字段，避免乐观并发噪声）；
   - 任务删除/归档：检查项随软删（purge 语义，无孤儿）。
4. 命令：`task_checklist_list/add/update/delete/reorder` + lib 注册；`get_task` 不内联 checklist（独立查询，避免热路径放大）。
5. 前端：
   - `TaskDetailPanel` 新「检查项」区：输入框回车追加、checkbox 勾选、行内编辑内容（blur 保存）、拖拽排序（复用 @dnd-kit 模式）+ 删除（两步确认）；
   - `TaskRow` 标题后 `2/5` 徽标（无检查项不显示）；
   - 已完成任务：检查项只读展示。
6. 搜索：检查项内容并入任务搜索索引（`search.upsert` body 追加 checklist 文本），保证「按子条目找任务」可达。

## Acceptance Criteria

- [x] 详详面板 3 步内完成 增/勾/删；上限 50 超出报错文案
- [x] 勾选全部不自动完成任务；完成任务后检查项冻结只读（单测）
- [x] 删除检查项后 sort_order 重排连续（单测）；reorder 语义与任务一致
- [x] 任务删除/归档后无孤儿检查项（单测：query 全表为空）
- [x] 检查项文本可被任务搜索命中（单测）
- [x] TaskRow 显示 n/m 进度；无检查项任务渲染零变化
- [x] cargo test / pnpm test / build 全绿；PRD/implement 勾选；父合同复核
- [x] docs：keyboard-shortcuts 无需改（无新全局键）；ui-layout-interaction.md 检查项一节

## 明确不做（本切片）

- 多层嵌套检查项（§9.3 明确一层）
- 检查项单独提醒/日期（拆分生成为主）
- AI 生成检查项（切片 7）
- 记忆/提醒侧检查项
