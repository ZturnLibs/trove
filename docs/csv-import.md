# 任务 CSV 导入导出（v1.4 切片 5）

CSV 用于**任务迁移**，不是无损备份。周期规则、提醒、记忆、剪切板请继续使用 [JSON 导出](./privacy-and-data.md) 或备份恢复。

## 列

导出列（`utf-8`，带表头）：

`title,notes,status,priority,list,due_date,due_time,tags`

- `status`：`todo` / `completed` / `archived`
- `priority`：`none` / `low` / `medium` / `high`
- `tags`：分号分隔
- `due_date`：`YYYY-MM-DD`；`due_time`：`HH:MM`

导入时识别常见别名（`标题`/`任务`/`Title`、`截止日期`、`清单`、`标签` 等）。无法匹配的清单名会进入收件箱，并在预览中列出。

## 流程

1. **预览**（`csv_preview_tasks`）：行数、字段映射、重复（同标题 + 同截止日期）、错误行、样例。不写库。
2. **导入**（`csv_import_tasks`）：有校验错误则拒绝；有重复时须 `skipDuplicates`。中途失败会删除本批已创建任务。
3. **批次**（`import_batches`）：记录创建的任务 ID。尚未被用户继续修改的条目可 **撤销**（`csv_undo_import`）。

上限 5000 行。

## IPC

| 命令 | 说明 |
|------|------|
| `csv_export_tasks` | 导出当前未删除任务 |
| `csv_preview_tasks` | 预览映射与错误 |
| `csv_import_tasks` | 确认导入 |
| `csv_import_batches` | 最近批次 |
| `csv_undo_import` | 撤销一批 |

设置页「备份与数据 → 任务 CSV」。

## 明确不做

- Things / Todoist 专用格式解析（可用其 CSV 导出后按列名映射）
- 导入周期任务或提醒
- 覆盖式导入（与 JSON 全量导入不同，CSV 只**追加**任务）
