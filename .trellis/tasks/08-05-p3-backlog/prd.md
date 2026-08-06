# P3 技术债与体验增强 backlog（p3-backlog）

## Goal

收尾 P3 聚合任务：把审计中的 P3 项整理为可追溯的 backlog（含推荐优先级），并实现其中两个低成本、高价值的「通知」快赢项（通知点击落地 + 通知正文上下文），其余保留为后续 backlog。

## P3 清单（来自 feature-gap-audit research/gap-audit.md）

| 项 | 说明 | 建议 |
|---|---|---|
| 通知点击落地（reminder://fired 无 listener） | 通知触发时主窗口无感知 | **本轮实现**（快赢） |
| 通知正文上下文 | 固定文案「任务提醒到期」无时间/备注 | **本轮实现**（快赢） |
| 周期任务不可编辑 | 创建后只能完成/跳过/延期 | 后续 backlog |
| 周期提醒 UI 仅「每天重复」 | weekly/monthly 仅 NL 可达 | 后续 backlog |
| 自定义清单无删除/重命名 | 缺命令与 UI | 后续 backlog |
| 查询无分页（任务/记忆） | 大列表性能隐患 | 后续 backlog |
| 任务页无页内搜索 | 需开全局面板 | 后续 backlog |
| 子任务/检查项 | 数据库无 parent 概念 | 后续 backlog（需迁移） |
| 搜索筛选维度少 | 按类型/来源/时间筛选 | 后续 backlog |
| 截图快速收藏 | 需新平台能力 | 后续 backlog（独立立项） |
| 存储占用管理 | 需新命令/UI | 后续 backlog |
| 菜单栏今日面板 | v1.1 承诺 | 后续 backlog（独立立项） |
| 命令面板命令/结果未分区 | 轻微体验 | 后续 backlog |
| 帮助菜单内容为空 | 均指向设置 | 后续 backlog |

## Requirements

- R1 后端 `scheduler.rs`：通知正文增加上下文——使用提醒的计划时间（如 `计划时间 {scheduled_at}`，去掉秒）替代固定文案。
- R2 前端 `MainShell.tsx` 监听 `reminder://fired` 事件：导航到 `/today`（沿用 `main://navigate`），让通知触发时主窗口落地到今日提醒。
- R3 不改变通知标题、调度、贪睡/完成等行为。

## Acceptance Criteria

- [ ] AC1 通知正文含计划时间上下文（后端）。
- [ ] AC2 通知触发时主窗口监听 `reminder://fired` 并导航 `/today`。
- [ ] AC3 `cargo check`、`pnpm typecheck`、`pnpm build` 通过；P3 其余项已记录为 backlog。

## Notes

- 轻量-中复杂度任务：PRD-only。改动为 Rust（scheduler.rs）+ 前端（MainShell.tsx）。
- P3 其余项在本 PRD 中登记，后续按需从 `.trellis/tasks/08-05-p3-backlog/` 派生独立任务。
