# 时区正确性（timezone-fix）

## Goal

去掉 `Asia/Shanghai` 硬编码，使自然语言解析（NL）产出的周期规则与菜单「新建提醒」使用**系统时区**，保证非上海时区用户的时间/周期正确。

## 背景（勘察确认）

- `nl_parse.rs:28`：`let timezone = "Asia/Shanghai".to_string();` → 解析出的 `ParsedCapture.timezone` 与周期规则（`daily_rule`，如「每天吃药」）携带错误的时区。
- `menu_bar.rs:77`：菜单「新建提醒」创建时 `timezone: Some("Asia/Shanghai".into())`。
- 前端 QuickWindow 创建提醒时都用 `Intl.DateTimeFormat().resolvedOptions().timeZone` 覆盖（QuickWindow.tsx:321/348/354），故**非周期**路径时区正确；问题集中在 NL 解析出的周期规则与菜单提醒。
- `parse_capture` 是纯函数（domain/nl_parse.rs:20），被 commands `nl_parse_capture`（mod.rs:470）调用。
- `iana-time-zone` crate 未在依赖中；需要新增（轻量、跨平台，返回当前 IANA 时区名）。

## Requirements

- R1 引入 `iana-time-zone` 依赖。
- R2 `parse_capture(input, timezone: &str)` 增加时区参数；`nl_parse_capture` 命令内部用 `iana_time_zone::get_timezone()` 解析系统时区（失败回退 `"Asia/Shanghai"`），再调 `parse_capture`。`ParsedCapture.timezone` 与周期规则的 timezone 使用系统时区。
- R3 `menu_bar.rs:77` 用系统时区（`iana_time_zone::get_timezone()`，失败回退）替换硬编码。
- R4 更新 `nl_parse.rs` 测试：为 `parse_capture` 传入固定时区（如 `"Asia/Shanghai"` 或 `"UTC"`），断言不变。

## Acceptance Criteria

- [ ] AC1 `parse_capture` 签名变更后编译通过，命令 IPC 签名不变（仍是 `nl_parse_capture(text)`）。
- [ ] AC2 系统时区非上海时，NL 解析出的周期规则 timezone 为系统时区（单测用注入时区验证）。
- [ ] AC3 菜单新建提醒使用系统时区。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 轻量任务：PRD-only。改动为 Rust（nl_parse.rs、commands/mod.rs、menu_bar.rs、Cargo.toml + Cargo.lock）。
- 不改变前端既有系统时区覆盖逻辑。
