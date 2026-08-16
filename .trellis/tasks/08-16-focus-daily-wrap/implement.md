# Implement: 专注模式与每日收尾

> **前置**：`08-16-gtd-workflow-states` Phase 3（今日重点）合并后再 start 本任务。  
> 每 Phase 对照 [`quality-bar.md`](../08-16-v1-3-gap-roadmap/quality-bar.md)。

---

## Phase 0 — 数据与 API（无 UI）

- [x] Migration `0011_focus_sessions.sql`
- [x] `FocusService` + `DailyWrapService`（application 层）
- [x] IPC 命令注册 + TS 类型
- [x] 单测：focus 状态机、daily_wrap snapshot
- [x] **门禁**：gtd-workflow-states 已归档或 feature branch 已合并

---

## Phase 1 — 专注模式

- [x] `FocusOverlay` 全屏组件
- [x] Today + TaskDetail 入口 + `⌘↵` / `Enter`（重点区）
- [x] 倒计时 + 到点通知
- [x] Esc 确认 / 完成 / 保持待办
- [x] EntityLink 相关区（复用 AttachmentsSection 模式）
- [x] 异常退出 + 启动横幅
- [ ] **极致门禁**：F1/F2/F3 场景通过

---

## Phase 2 — 每日收尾

- [x] `DailyWrapWizard` 分步 Modal
- [x] Step1 整合 DeferPicker / 等待表单
- [x] Step2–4 快照展示
- [x] recent-actions 逐步撤销
- [x] 「今日已收尾」状态
- [ ] **极致门禁**：F4/F5；无 silent 批量变更

---

## Phase 3 — 文档与归档

- [x] `keyboard-shortcuts.md` 更新
- [x] `ui-layout-interaction.md` 专注/收尾节
- [x] trellis-check + quality-bar 全勾

---

## PR 建议

| PR | 内容 |
| --- | --- |
| PR1 | Phase 0 + Phase 1 |
| PR2 | Phase 2 + Phase 3 |

---

## Rollback

- 0011 为独立表；移除 overlay 路由即可隐藏 UI，数据保留
