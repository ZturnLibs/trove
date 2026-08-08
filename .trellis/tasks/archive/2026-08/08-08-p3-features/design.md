# P3 功能技术设计

## 背景

P3 三项能力（清单管理、任务搜索、剪切板筛选）已在 `feat-improve` 分支合并至 `main`。本任务在 `feat-update` worktree 同步代码后做验收确认与测试补全。

## 4.4 自定义清单删除/重命名

### 后端（`TaskService`）

- `update_list(id, name)` — 重命名自定义清单
- `delete_list(id, disposition)` — 删除清单，支持三种未完成任务处置：
  - `MoveToInbox` — 移入收件箱
  - `CompleteAll` — 全部标记完成
  - `DeleteAll` — 删除所有任务
- `undo_delete_list(result)` — 撤销删除，恢复清单及任务归属
- `list_todo_count(list_id)` — 删除前查询未完成任务数

### 前端（`TasksPage`）

- 右键清单上下文菜单：重命名（`prompt`）、删除
- 删除时若有未完成任务，弹出对话框选择处置方式
- 删除成功后写入 `useRecentActions` 支持撤销

## 4.5 任务页内搜索

### 后端

- `TaskQuery.search` 对标题与备注做 `LIKE` 模糊匹配，与 `list_id`、`status`、`priority`、`tag_id`、智能列表条件叠加（AND）

### 前端

- 搜索框 state 传入 `ipc.taskQuery`，与现有筛选器共用 query key 触发刷新

## 4.6 剪切板来源与时间筛选

### 后端

- `ClipboardQuery.source_app` — 精确匹配来源应用
- `ClipboardQuery.date_from` / `date_to` — 按 `date(created_at)` 范围筛选
- `list_source_apps()` — 返回历史中出现过的来源应用列表

### 前端

- 来源下拉：从 `clipboardListSourceApps` 填充
- 时间下拉：`all` / `7d` / `30d`，前端计算 ISO 日期传给后端

## 数据流

```
TasksPage / ClipboardPage
  → ipc.* (Tauri commands)
  → TaskService / ClipboardService::query
  → SQLite
```
