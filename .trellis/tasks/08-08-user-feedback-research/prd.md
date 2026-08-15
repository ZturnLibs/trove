# 个人工作台用户需求全网调研

## Goal

从全网公开渠道收集、归纳用户对「个人工作台 / 本地优先效率工具」的真实反馈与需求，形成可指导 Trove 产品迭代的调研报告。

## Scope

- **产品锚点**：Trove — 本地优先个人工作台（任务、提醒、记忆、剪切板）
- **调研范围**：竞品用户反馈、社区讨论、开源 Issue、产品评测、效率方法论社区
- **不在范围**：付费用户访谈、定量问卷、Trove 自有用户数据（暂无公开渠道）

## Requirements

1. 覆盖至少 5 类信息渠道（HN/Reddit/GitHub Issues/中文社区/产品评测）
2. 按「用户真实诉求主题」归纳，而非按竞品罗列
3. 每条高优先级需求附可溯源来源（URL 或社区帖子）
4. 与 Trove 现有定位（`docs/post-v1-iteration-design.md`）对照，标注契合/冲突/待验证
5. 产出优先级建议（P0–P3）及「明确不做」边界

## Acceptance Criteria

- [x] 调研报告写入 `research/user-feedback-report.md`
- [x] 报告含：方法、主题归纳、Trove 映射、优先级建议、来源索引
- [x] PRD 与报告一致，可作为后续 PRD/路线图输入

## Notes

- 本任务为**研究交付**，不修改业务代码
- 与内部 gap-audit（`.trellis/tasks/archive/2026-08/08-05-feature-gap-audit/`）互补：gap-audit 偏实现差距，本报告偏外部用户声音
