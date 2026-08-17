# Implement: 统一动作层

- [x] `domain/workbench_action.rs` + mod 导出
- [x] `application/workbench_actions.rs` + dispatch
- [x] 重构 `url_scheme.rs` 使用 dispatch（lib.rs 传入 AppState）
- [x] IPC `workbench_action_dispatch` + lib 注册
- [x] 单测：From 转换、确认门禁
- [x] `docs/action-layer.md`
- [x] `cargo test` 97/97

## 切片 2 — 本地 CLI

- [x] `trove-cli` 二进制（clap）
- [x] `trove-action:` 单实例 / 冷启动协议 + 响应文件
- [x] `docs/cli.md`

## 切片 3 — 规则自动化

- [x] `migrations/0017_automation.sql`
- [x] `domain/automation.rs` + `application/automation.rs`
- [x] 创建/更新/收藏/提醒触发挂钩 + 防递归
- [x] `AppSettings.automation_enabled` + 设置页规则区块
- [x] IPC：`automation_*` 命令
- [x] `docs/automation.md`
- [x] `cargo test`

## 切片 4 — macOS 快捷指令

- [x] 动作层只读查询：今日 / 逾期 / 收件箱 / 清单 / 记忆 / 片段
- [x] `trove-cli query …` + `--json`
- [x] `docs/shortcuts.md`
- [x] `cargo test`
