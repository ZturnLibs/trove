# 托盘今日面板

## Goal

补齐 v1.1 承诺的菜单栏/托盘「今日视图」差距，对标 Things 3 菜单栏快速处理，减少打开主窗口的切换成本。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Things 菜单栏 Today | 托盘仅打开/快速记录/剪切板/设置 |
| TickTick 小组件 | 无 |

## Requirements

1. **入口**：托盘左键弹层或子菜单展示今日摘要（二选一，推荐弹层与现有 quick 锚定一致）
2. **内容**：逾期数、今日任务/提醒摘要、下一条即将触发提醒
3. **操作**：完成、延期（预设）、打开详情（导航主窗口）
4. **性能**：打开 < 200ms；数据与 TodayPage 一致（同一 query）
5. **平台**：macOS + Windows

## Acceptance Criteria

- [ ] 不打开主窗口可完成至少一项今日任务或提醒
- [ ] 数字与 TodayPage 一致
- [ ] 托盘现有菜单项不受影响
- [ ] 无今日项时空状态文案友好

## 复杂度

**Medium** — 主要是 UI 与 IPC；可复用 TodayPage 查询逻辑。

## Notes

- v1.1 余留项（gap-audit P3 → 现仍为差距）
- Wave C 第一项；可与 gtd-workflow-states 解耦先行
