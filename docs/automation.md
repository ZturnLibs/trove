# 规则自动化（v1.4 切片 3）

本地规则引擎：在特定事件发生后，按条件执行已注册动作。不解释或运行任意脚本。

## 触发器（首版）

| 触发器 | 说明 |
|--------|------|
| `taskCreated` | 创建任务后 |
| `reminderCreated` | 创建提醒后 |
| `memoryCreated` | 创建记忆后 |
| `clipboardFavorited` | 剪切板条目被收藏后 |
| `reminderFired` | 提醒触发（系统通知已展示）后 |
| `taskMovedToList` | 任务移入清单后（可选限定目标清单） |
| `taskTagAdded` | 任务新增标签后（可选限定标签名） |

## 条件（AND）

- `titleContains` / `bodyContains`：关键词（可忽略大小写）
- `entityType`：条目类型
- `listId` / `hasTag` / `priority` / `sourceApp`
- `weekday`：本地工作日（0=周一 … 6=周日）
- `timeRange`：本地时间范围 `HH:MM`

空条件列表表示始终匹配（触发器匹配即可）。

## 动作（首版）

| 动作 | 适用实体 |
|------|----------|
| `setPriority` | 任务 |
| `moveToList` | 任务 |
| `addTag` | 任务 |
| `pinMemory` | 记忆 |
| `notify` | 任意（本地通知） |

首版动作**不创建**新实体，避免规则递归触发。

## 安全与运维

- **全局开关**：设置 → 规则自动化 →「启用规则自动化」(`automationEnabled`)
- **单条暂停**：每条规则可单独 `enabled = false`
- **防递归**：规则执行期间不再触发新规则（thread-local depth）
- **执行日志**：`automation_runs` 表；设置页展示最近 20 条
- **试运行**：IPC `automation_dry_run(ruleId, sampleEvent)` 仅评估，不写实体

## IPC

| 命令 | 说明 |
|------|------|
| `automation_list` | 列出规则 |
| `automation_create` | 创建规则 |
| `automation_update` | 更新规则 |
| `automation_delete` | 软删除规则 |
| `automation_set_enabled` | 单条启停 |
| `automation_runs_list` | 执行日志 |
| `automation_dry_run` | 试运行 |

## 数据表

- `automation_rules`：规则定义 JSON（trigger + conditions + actions）
- `automation_runs`：执行时间、目标、结果、脱敏错误摘要

Migration：`0017_automation.sql`

## 与动作层关系

规则动作在应用层直接调用 `TaskService` / `MemoryService` 等，与 [action-layer.md](./action-layer.md) 的 `WorkbenchAction` 并行存在。未来可将常用动作收敛到统一动作层。

## 明确不做（本切片）

- 任意 JavaScript / Shell 执行
- 静默创建任务/提醒/记忆
- 历史数据批处理回溯
