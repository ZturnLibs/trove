# Design: v2.0 任务检查项数据模型（切片 6）

## 1. 模块边界

```
migrations/0020_task_checklist_items.sql     表 + 索引
domain/task.rs                               ChecklistItem / TaskChecklist / CheckUpdate
application/tasks.rs                         checklist_* 方法（TaskService 内聚，同事务触碰父任务 updated_at）
commands/mod.rs                              5 个 IPC
client.ts                                    类型 + 封装
TaskDetailPanel                              检查项区（新增 ChecklistSection 子组件）
TaskRow                                      进度徽标
search.rs 消费侧                             tasks.rs 在 checklist 变更后重写索引（body += checklist 文本）
```

## 2. 关键决策

- **归属 TaskService**：检查项生命周期与任务强绑定（冻结/级联软删），跨服务编排得不偿失。
- **独立命令而非嵌入 task_get**：今日/列表热路径一次查询都不多付；UI 在详情面板打开时单独拉。
- **父任务 updated_at 触碰、revision 不 bump**：列表"最近更新"语义生效；乐观并发（revision 冲突检测）不受检查项操作干扰。
- **搜索索引组合**：`search.upsert(Task, id, title, notes + "\n" + checklist_text)`——既有重建逻辑（rebuild_all）同法，无新表。

## 3. 并发与事务

- add/update/delete/reorder 单条 SQL 各自原子；normalize（删除后重排）在事务内两步（读活跃项→批量 UPDATE）。
- 任务 completed/archived 冻结：所有写方法先 `get_task` 校验状态，统一错误「任务已完成，检查项不可修改」。

## 4. UI 状态机

```
ChecklistSection(taskId)
  items = query(["tasks","checklist",taskId])
  [输入框 Enter] → add → invalidate
  [checkbox] → update(checked) → invalidate（含 TaskRow 进度：invalidate ["tasks"]）
  [内容编辑 blur] → update(content)
  [拖拽 onDragEnd] → reorder(orderedIds)
  [删除 ConfirmButton] → delete → invalidate
  task.status === completed → 全控件 disabled，只读列表
```

进度徽标：`TaskChecklist { total, checkedCount }` 由 `checklist_list` 返回；TaskRow 需要 `total>0` 才显示 `checked/total`——由任务列表查询**不**附带（避免 N+1），改为 TaskRow 内独立 query（列表行 20 条内可接受）或复用已有 `tasks` 缓存拓展。**决策：TaskRow 独立 `useQuery(["tasks","checklist",id], enabled: !!id)`，staleTime 30s**——实现简单、无后端改动。

## 5. 风险

| 风险 | 对策 |
| --- | --- |
| 50 上限体验 | 超限文案温和提示（「检查项最多 50 条，考虑拆成子任务」——文案遵循 empty-states 语气） |
| 拖拽与行内编辑冲突 | 拖拽手柄独立于文本区 |
| 索引重建成本 | checklist 变更仅在单任务维度重写一条索引行 |

## 6. 回滚

迁移 0020 增量表；服务/命令/UI/docs 独立 commit；revert 迁移不影响 v1.x 数据。
