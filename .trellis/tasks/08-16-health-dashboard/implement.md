# 健康仪表盘 — 实现记录

## Phase 0 — 聚合 API

- [x] `domain/health.rs` — 快照类型
- [x] `application/health_dashboard.rs` — 备份 / 存储 / 提醒 / 任务 / 剪贴板聚合
- [x] IPC `health_dashboard_snapshot`
- [x] 单测：基础快照 + 提醒 7/30 天本地日期窗口

## Phase 1 — 前端

- [x] `/health` 路由 + `HealthDashboardPage`
- [x] 设置「工作节奏 → 打开健康仪表盘」
- [x] `ipc.client.ts` 类型

## Phase 2 — 文档

- [x] `docs/ui-layout-interaction.md` §7.1.4
- [x] `docs/keyboard-shortcuts.md`

## 验证

- [x] `cargo test` 73/73
- [x] `pnpm typecheck`
- [x] `pnpm test:unit` 35/35

## 手动验收（待勾选）

- [ ] 备份数字与设置页手动备份后一致
- [ ] 存储占用与磁盘大致吻合（见页内统计口径说明）
- [ ] 提醒比例与 scheduler 行为目测一致
