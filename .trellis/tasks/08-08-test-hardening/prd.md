# 跨版本测试加固（v1.2.1 · 4.7）

## 背景

v1.2.1 已交付分页与周期提醒 UI；`scheduler.rs`、迁移备份路径、提醒 reconcile/snooze 边界仍缺少自动化覆盖。

## 范围

- 提醒服务：`reconcile_on_startup`（单次 overdue 计数、周期去重 inferred_missed）
- 提醒服务：`snooze_occurrence` 后 `due_occurrences` 边界
- 数据库：待执行迁移前创建备份文件
- 前端：`useRecentActions` 栈上限与 pop 行为

不在范围：Windows OCR（4.3）、scheduler Tauri 通知集成测试。

## 验收标准

1. `cargo test` 新增用例全部通过
2. `pnpm test:unit` 新增用例全部通过
3. CI 现有 workflow 无需改动即可绿
