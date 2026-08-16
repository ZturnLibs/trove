# 任务列表支持鼠标左键按住拖动排序

## Goal

在任务列表视图中，支持鼠标左键按住任务行并拖动，调整任务在列表中的顺序。

## Background

- 现有「上移 / 下移」按钮依赖选中任务（`TasksPage.tsx` reorderMutation → `ipc.taskReorder(orderedIds)`），后端 `task_reorder` 已存在。
- 交互规范 `docs/ui-layout-interaction.md:269` 已声明「列表支持拖拽排序（仅当前视图排序值）」，但未实现。

## Requirements

1. 任务列表（`TasksPage` 普通清单视图）中，鼠标左键按住 `TaskRow` 可拖动排序。
2. 拖拽过程中提供可视反馈（被拖行置顶/置灰，目标位置显示插入指示线）。
3. 松开后调用现有 `ipc.taskReorder` 持久化，并刷新列表。
4. 智能列表（`smart !== "none"`）**不启用**拖拽排序（条件视图，非数据副本）。
5. 拖拽期间不触发 `TaskRow` 的 `onClick` 选中逻辑。
6. 不引入新依赖（复用现有栈，无 dnd 库）。

## Constraints

- 前端单层改动：`TasksPage.tsx` / `TaskRow.tsx`，后端无改动。
- 保持键盘优先交互不受影响（现有快捷键与上移/下移按钮保留）。

## Acceptance Criteria

- [ ] 普通清单视图中按住任务行拖动，可实时预览目标插入位置
- [ ] 松开后顺序持久化（刷新后仍保持新顺序），调用 `taskReorder`
- [ ] 智能列表视图下拖拽无效，不出现拖拽手势
- [ ] 拖拽完成不误触发任务选中 / 双击重命名
- [ ] 无新 npm 依赖；typecheck 与现有测试通过

## Notes

- 轻量级任务，PRD-only，不写 design.md / implement.md。
- 与内部 gap-audit / 调研报告无冲突（排序为交互增强，非新数据模型）。
