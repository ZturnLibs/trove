# Implement: 每周回顾

---

## Phase 0 — 数据与 API

- [x] Migration `0012_weekly_review.sql`
- [x] `WeeklyReviewService` + `ReviewSession`
- [x] IPC：`weekly_review_snapshot/start/complete/last_completed`
- [x] 单测：snapshot + complete

---

## Phase 1 — UI

- [x] `/weekly-review` 页面 + 7 类信号卡片
- [x] 逐项完成 / 收藏剪切板 + recent-actions
- [x] 详情侧栏 + 跳转入口
- [x] Today / 设置入口
- [x] 「完成本次回顾」+ 距上次间隔

---

## Phase 2 — 文档

- [x] `keyboard-shortcuts.md` / `ui-layout-interaction.md` 补充

---

## 手动验收

- [ ] 七类数字与对应列表页一致
- [ ] 处理后卡片计数刷新
- [ ] 完成回顾后再次进入显示间隔天数
