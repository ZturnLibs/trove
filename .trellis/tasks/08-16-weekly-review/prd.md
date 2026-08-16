# 每周回顾卡片

## Goal

对标 GTD/Everdo 每周回顾与 deariary「reconstruction tax」痛点：一屏呈现待整理信号，逐项处理，不生成绩效分数。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Everdo 结构化 weekly review | 无 |
| Todoist 周回顾（部分付费） | 无 |
| 完成/延期/积压统计 | 无聚合 UI |

## Requirements

1. **回顾入口**：设置或今日页进入「每周回顾」
2. **卡片内容**（确定性 SQL）：未整理收件箱、逾期任务、等待跟进、长期未修改活跃任务、近 7 天完成/取消、即将到来周期任务、大体积未收藏剪切板
3. **逐项处理**：每项可跳转原条目并完成/延期/推迟/等待/归档
4. **完成记录**：记录本次回顾完成时间（`ReviewSession`）
5. **可视化**：周报卡片组件；任务名可点击跳转；**禁止**效率评分/排名

## Acceptance Criteria

- [ ] 一屏展示上述 7 类信号，数字与列表页一致
- [ ] 逐项处理后对应卡片计数实时更新
- [ ] 回顾完成时间持久化；再次打开可看到距上次间隔
- [ ] 可选 AI 摘要**不在本任务范围**（v2.0 门槛后另开）

## 复杂度

**Medium** — 可 PRD + 轻量 `implement.md`；聚合查询可与 `health-dashboard` 共享设计。

## Notes

- 依赖 gtd-workflow-states 的等待/推迟筛选
- 复用 health-dashboard 部分 SQL 时可协调接口，避免重复
- Wave A 第三项
