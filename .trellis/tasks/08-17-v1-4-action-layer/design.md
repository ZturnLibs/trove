# Design: 统一动作层（切片 1）

## 分层

| 层 | 职责 |
| --- | --- |
| `domain/workbench_action.rs` | 动作枚举、来源、结果、URL 转换 |
| `application/workbench_actions.rs` | 分发：窗口聚焦 + 事件 emit；变异动作确认门禁 |
| `application/url_scheme.rs` | 解析 URL → 调用 dispatch |
| `commands/mod.rs` | IPC `workbench_action_dispatch` |

## 动作类型（切片 1）

- `Navigate { path }` → 主窗 + `main://navigate`
- `OpenSearch { query }` → quick 窗 + `quick://set-search-query`
- `CreatePreview { kind, title, notes, due_date, fire_at }` → 主窗 + `url-scheme://pending-create`
- `CreateTask` / `CreateReminder` / `CreateMemory` / `CompleteTask` — **枚举预留**；未 `confirmed` 时返回 `Rejected`

## 安全

- 外部来源（UrlScheme、未来 Cli）的变异动作必须 `confirmed: true` 才持久化
- `dry_run: true` 时仅返回将执行的动作摘要，不写库

## 兼容

- 不修改 `UrlSchemeAction` 对外 JSON 形状
- deep-link 注册与解析逻辑不变
