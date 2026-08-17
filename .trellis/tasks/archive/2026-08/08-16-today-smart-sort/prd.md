# 今日智能排序建议

## Goal

对标 Todoist Smart Schedule 的轻量本地版：在今日页给出可解释的任务顺序建议，用户可采纳或忽略，不自动改归属。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Todoist 智能排期 | 无 |
| Things 手动排序 | 有拖拽，无建议 |

## Requirements

1. **算法（确定性）**：综合截止时间、优先级、历史延期次数、提醒时间输出建议序
2. **理由文案**：每项附一句话（如「今天到期」「已延期 2 次」）
3. **采纳**：「采纳建议顺序」按钮应用排序；之后仍可拖拽调整
4. **解耦**：建议不影响今日重点 membership 或任务 status
5. **数据**：延期次数需轻量埋点（可重建，不纳入主备份 per post-v1 §10）

## Acceptance Criteria

- [ ] 今日页展示建议顺序与理由；空列表/单条 gracefully 降级
- [ ] 采纳后顺序与手动拖拽共存（SavedView/排序字段一致）
- [ ] 算法纯本地；关闭建议后 UI 恢复纯手动
- [ ] 与 gtd-workflow-states 今日重点可同时存在且不冲突

## 复杂度

**Medium** — 需延期统计存储方案（design 一节即可）。

## Notes

- 依赖 gtd-workflow-states 完成后体验最佳；可并行开发
- Wave B 末项
