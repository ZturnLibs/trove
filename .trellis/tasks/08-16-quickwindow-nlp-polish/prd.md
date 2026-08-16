# 快速捕获与 NLP 体验加深

## Goal

对标 Raycast #15993（Things 级字段跳转）与 Todoist NLP：在已有 QuickWindow/NL 基础上降低记录成本，巩固 Trove 核心差异化。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Raycast Reminders NLP + Cmd+字段跳转 | NL 有，字段跳转弱 |
| Todoist 自然语言日期/项目 | 部分语法支持 |
| 命令/结果分区 | QuickWindow 合并列表 |

## Requirements

1. **NLP 扩展**：支持 `#tag`、`p1/p2/p3`、相对日期、weekly/monthly recurrence 与 RecurrencePicker 回填互通
2. **字段跳转**：QuickWindow 捕获态 Tab/Cmd+数字 切换 标题/日期/时间/标签/优先级
3. **命令分区**：搜索模式下命令与内容命中分区展示（v1.1 设计余留）
4. **提醒侧重**：独立提醒创建路径 NLP 与任务对齐
5. **可发现性**：设置/帮助中列出 QuickWindow 语法速查

## Acceptance Criteria

- [ ] `明天下午 #工作 p1 回复客户` 一次解析并创建正确任务
- [ ] 键盘可在捕获表单字段间跳转，无需鼠标
- [ ] 搜索模式命令与结果视觉分区清晰
- [ ] 现有 NL 用例不回归（补 vitest/fixture）

## 复杂度

**Light–Medium** — 以前端 + `nl_parse.rs` 为主；PRD-only 可启动。

## Notes

- 外部证据：user-feedback-report §3.1、§3.6
- Wave C；可与 tray-today-panel 并行
