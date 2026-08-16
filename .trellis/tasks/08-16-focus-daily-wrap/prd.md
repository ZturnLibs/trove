# 专注模式与每日收尾

## Goal

对标 Everdo「单条 Focus Review」与 Will Be Done 温和收尾：帮助用户聚焦单任务、下班前逐项处理未完成事项，降低 guilt-trip 与 reconstruction tax。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Everdo weekly review focus 模式 | 无 |
| 番茄/专注计时（TickTick 等） | 无 |
| 每日收尾向导 | 无 |

## Requirements

1. **专注模式**：从今日重点或任意任务进入；展示当前任务说明与关联记忆/附件；可选倒计时，结束仅提醒不强制停止
2. **退出行为**：可完成任务、记录简短进展备注、或保持原状态；异常退出不自动标记完成
3. **每日收尾**：可跳过引导流程——未完成重点 → 逐项选择保留/延期/推迟/等待/取消 → 预览明日到期 → 收件箱清理 → 当日完成摘要
4. **批量操作**：任何批量变更须用户逐项或批量确认；支持撤销栈

## Acceptance Criteria

- [ ] 专注会话可开始/结束/异常退出，任务状态不被误改
- [ ] 倒计时结束有本地通知，不锁屏或阻断系统
- [ ] 每日收尾每步可跳过；批量延期须确认
- [ ] 进展备注与 FocusSession 记录可本地查询（供 weekly-review 复用）
- [ ] 可选分心洞察（前台应用切换）默认关闭，见 roadmap §5.6 可拆 follow-up

## 复杂度

**Complex** — [`design.md`](./design.md) 与 [`implement.md`](./implement.md) 已就绪；依赖 gtd-workflow-states Phase 3 后再 `task.py start`。

## 体验极致

遵循父任务 [`quality-bar.md`](../08-16-v1-3-gap-roadmap/quality-bar.md)。对标 F1–F5 见 design §1。

## Notes

- 依赖 `08-16-gtd-workflow-states` 的今日重点与推迟/等待语义
- Wave A 第二项
