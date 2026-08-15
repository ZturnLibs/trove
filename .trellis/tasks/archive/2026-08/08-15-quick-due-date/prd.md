# 任务行截止日期快捷修改（今天/明天/后天菜单）

## Goal

在任务行右侧的截止日期/时间显示区（`TaskRow`）增加快捷修改能力：点击弹出菜单，支持快捷选定「今天 / 明天 / 后天 / 后天之后（+3 天等）/ 清除日期」，也可自定义日期与时间。功能在 TaskRow 组件层通用，任务列表、今日、收件箱所有使用 TaskRow 的列表统一生效。

## Background

- `TaskRow.tsx:148-158` 展示 `task.dueDate`（+ `dueTime`），当前只读，无法从列表行直接修改。
- 后端 `update_task`（`tasks.rs:360`）为全量更新（需 title/notes/priority/listId/dueDate/dueTime/tagNames）；`TaskDetailPanel` 已有完整调用范例（`ipc.taskUpdate` + `invalidateQueries(["tasks"])`）。
- 日期字符串格式：`dueDate` 为 `YYYY-MM-DD`，`dueTime` 为 `HH:MM`（`validate_due_date`/`validate_due_time`）。

## Requirements

1. 点击 `TaskRow` 右侧的时间文本，弹出快捷菜单（不触发行选中，`stopPropagation`）。
2. 菜单选项：
   - 「今天」「明天」「后天」快捷项（按本地时区计算日期）
   - 「+3 天」及以上的通用增量项（至少覆盖 +3/+7）
   - 自定义日期（`<input type="date">`）+ 可选时间（`<input type="time">`）
   - 「清除日期」项
3. 选择后调用 `ipc.taskUpdate` 更新 `dueDate`（保留现有 `dueTime` 除非用户修改），成功后 `invalidateQueries(["tasks"])` 刷新。
4. 菜单为浮层，点击外部 / Esc 关闭；菜单项点击不冒泡到行。
5. 已有到期日期的行显示菜单项时，当前日期高亮（可选，若有现成选中态模式）。

## Constraints

- 前端改动限 `TaskRow.tsx`（通用交互）与必要的共享浮层；不改后端。
- 复用现有 `ipc.taskUpdate` 全量更新（构造完整 `UpdateTaskInput`：保留当前 title/notes/priority/listId/tagNames，仅替换 dueDate/dueTime）。
- 菜单浮层样式遵循现有 listMenu 浮层模式（`TasksPage.tsx` 的 listMenu 参考：`fixed z-50 ... bg-surface py-1 shadow-lg`）。
- 拖拽（SortableTaskRow）、双击重命名、Enter/Space 选中不受影响。

## Acceptance Criteria

- [ ] 点击任务行右侧时间弹出快捷菜单，不触发行选中
- [ ] 「今天/明天/后天」快捷项按本地日期正确设置 `dueDate`
- [ ] 自定义日期/时间可编辑保存；清除日期可移除 `dueDate`
- [ ] 保存后列表立即反映新日期（invalidate 刷新），不闪回
- [ ] 菜单点击外部或 Esc 关闭
- [ ] 任务列表 / 今日 / 收件箱三处均生效
- [ ] 拖拽排序、双击重命名、键盘选中不受影响
- [ ] `pnpm typecheck` 与 `pnpm test:unit` 通过

## Notes

- 轻量任务，PRD-only。
- 与 taskReorder 乐观更新互不冲突（本次不引入乐观更新，invalidate 即可；闪回问题只影响拖拽）。
