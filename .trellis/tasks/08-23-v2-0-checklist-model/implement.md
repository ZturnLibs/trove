# Implement: v2.0 任务检查项数据模型（切片 6）

## 1. 数据与领域层

- [x] `migrations/0020_task_checklist_items.sql` + MIGRATIONS/断言 19→20（db/backup 两处测试）
- [x] domain：`ChecklistItem`/`TaskChecklist`/`ChecklistUpdateInput` + content 校验（trim 非空 ≤200 字）
- 验证：`cargo test --lib infrastructure::db domain::task`

## 2. 服务层（TaskService）

- [x] `checklist_list/add/update/delete/reorder` + 50 上限 + 冻结校验 + 级联软删 + normalize 重排
- [x] checklist 写操作后重写任务搜索索引（body 拼接 checklist 文本）
- [x] 单测：增删改勾、排序重排、冻结、级联、上限、搜索命中
- 验证：`cargo test --lib application::tasks`

## 3. IPC + 前端

- [x] 命令 5 个 + lib 注册；client.ts 类型/封装
- [x] `ChecklistSection`（输入追加/勾选/行内编辑/拖拽/两步删除/完成只读）+ TaskDetailPanel 接入
- [x] `TaskRow` 进度徽标（独立 query，staleTime）
- [x] `ui-layout-interaction.md` 检查项小节
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 4. 收尾

- [x] PRD/implement 勾选；父任务全局合同复核（本切片非 AI，重点 1/4/9 条）
- [x] `cargo test` 全绿（含已知 flaky 豁免记录）

## Review gates

1. 步骤 2 后：级联与冻结语义代码审查
2. 步骤 3 后：检查项区键盘流（Enter 追加/Tab 切换）
3. 完成后：finish + archive
