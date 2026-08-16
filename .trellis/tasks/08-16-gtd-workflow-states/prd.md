# GTD 工作流：今日重点 / 推迟显示 / 等待事项

## Goal

补齐与 Everdo、TickTick GTD 工作流对标的核心差距：让用户能标记今日重点、推迟任务显示、记录等待跟进，而不依赖日历或协作功能。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Everdo 等待/跟进日 | 无 |
| TickTick 稍后/隐藏至某日 | 无（仅有延期改 due） |
| Things Today 手动排序 | 有列表排序，无「重点」概念 |

## Requirements

1. **今日重点**：用户从任意待办选入今日重点，手动排序；默认建议 3–5 项，不强制上限
2. **推迟显示（availableAt）**：显示日期前任务不出现在今日/活跃列表，仍可在全部任务与搜索中找到
3. **等待事项**：任务可标记等待中，记录 `waitingFor` 文本与可选跟进日；跟进日到今日视图但不自动变回普通待办
4. **冲突提示**：截止日期早于显示日期时保存须提示
5. **筛选统一**：`query_tasks`、TodayPage、InboxPage、智能列表对「活跃任务」定义一致

## Acceptance Criteria

- [ ] 用户可选择、排序、完成今日重点，不改变任务原 list/status 归属
- [ ] 推迟任务在显示日前不进入活跃视图，搜索可命中
- [ ] 等待任务在跟进日出现在今日/跟进区，保留 waitingFor 信息
- [ ] 显示日期与截止日期冲突时有明确 UI 提示
- [ ] 迁移脚本可升级现有 DB；备份/恢复/导出兼容新字段

## 复杂度

**Complex** — `design.md` 与 `implement.md` 已就绪；`task.py start` 前请审阅 Phase 0 迁移方案。

## 体验极致

遵循父任务 [`quality-bar.md`](../08-16-v1-3-gap-roadmap/quality-bar.md)。对标场景 S1–S4 见 [`design.md`](./design.md) §1。

## Notes

- 设计依据：`docs/post-v1-iteration-design.md` §7.3–§7.4
- 建议 Wave A 第一项；为 `focus-daily-wrap`、`weekly-review` 提供数据基础
