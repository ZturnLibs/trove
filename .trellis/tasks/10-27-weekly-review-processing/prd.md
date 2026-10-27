# 周回顾处理模式（单条出牌）

## Goal

把每周回顾从「统计仪表盘」升级为「可清空的流程」：在回顾页提供**逐条出牌**处理模式，把逾期 / 收件箱积压 / 等待跟进 / 僵滞活跃四条队列一次一条地处理掉（完成 / 延期 / 转等待 / 跳过），全程键盘可驱动，结束产出处理统计。解决「复盘看得到问题却清不掉」（GTD/Everdo 共识，见 `archive/2026-08/08-08-user-feedback-research/research/user-feedback-report.md` §3.4）。

## Background（证据锚点）

- 会话模型已存在：`weeklyReviewStart/Complete`（`WeeklyReviewPage.tsx:145-167`），summary 字段开放（`ReviewCompleteInput.summary`）。
- 四条队列字段已就绪：`WeeklyReviewSnapshot`（`client.ts:541-556`）。
- 详细调研：`research/review-page-survey.md`。

## Requirements

### R1 处理模式入口与队列

- 回顾页新增「开始处理」按钮（有任一队列非空即可用）；进入后按 **逾期 → 收件箱 → 等待跟进 → 僵滞** 顺序逐条出牌，头部显示 `第 N / M 条 · 队列名`。
- 出牌卡呈现：标题、清单、到期/逾期天数、说明摘录；「查看详情」开既有 `TaskDetailPanel`。

### R2 四键出牌

- **完成**（`taskComplete`，recurring 既有语义）/ **延期**（`taskSetDefer`，默认 +1 天，可选日期）/ **转等待**（打开既有 `TaskDetailPanel` 等待流程，`applyMarkWaiting` 同款）/ **跳过**（不动任务，仅出牌）。
- 每次出牌后自动进入下一条；误操作走既有撤销 toast（`useRecentActions`）。
- 键盘：`1/2/3/4` 对应四键，`Esc` 退出处理模式（保留会话）。

### R3 统计与收尾

- 队列清空或手动结束时展示「本周期处理 N 项（完成 x / 延期 y / 转等待 z / 跳过 w）」，计入 `weeklyReviewComplete` summary。
- 处理模式内每次变更失效 `["tasks"]` 与 `["weekly-review"]`（快照计数实时递减）。

## Acceptance Criteria

- [ ] 四条队列各≥1 条时，处理模式按序出牌、头部进度正确；单队列清空自动切换下一队列
- [ ] 四键分别产生对应持久化效果（刷新后保持）；撤销 toast 可回滚最近一次
- [ ] 全键盘可完成一次完整处理（1-4 + Esc）；重复触发无副作用
- [ ] 结束统计数字与实际动作一致并出现在回顾完成态
- [ ] 队列全空时「开始处理」置灰；处理中他窗改动任务不崩溃（出牌前校验）
- [ ] `pnpm typecheck`、`pnpm test:unit` 全绿；出牌动作计数抽纯函数并加 vitest

## Out of Scope

- 出牌卡内的行内标题/清单编辑（引 DetailPanel 承担）
- 处理进度跨会话恢复；移动端
- 自定义队列顺序与过滤

## Open Questions（设计阶段决策，见 research）

- 会话中断恢复是否 v1 需要（倾向否）
- summary 统计键名规范（与 AI 周摘要的消费方对齐）

## Notes

- 复杂任务：进入实现前需补 `design.md` + `implement.md`（本 PRD 为种子，用户审完排期）。
- 遵循温和原则：无 guilt 文案，跳过非失败。
