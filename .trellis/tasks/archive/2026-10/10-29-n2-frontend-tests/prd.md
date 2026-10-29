# N2 前端测试覆盖补强——纯决策逻辑抽取与 vitest

## Goal

Roadmap N2（P2）：现有 63 条 unit 集中于纯函数与 lib；本轮 e2e 调试暴露的三处交互语义
（出牌队列状态机、看板拖拽分发分支、详情面板保存语义）只有端到端覆盖，改 UI 时缺快速反馈。
vitest 环境为 node（无 jsdom），按仓库既有惯例（taskTree/processingSummary/splitBoardColumns
纯函数模块）**抽取纯决策函数**并以 vitest 覆盖。

## Requirements

- `boardDrag.ts`（新）：从 BoardPage.handleDragEnd 抽出拖拽分发纯函数——
  跨列移动 / 同列排序 / 完成列禁止重排 / drop-on-self 与列 droppable no-op
- ProcessingSession：抽出队列出牌决策（动作→任务状态映射、已完结跳过、队列推进/结束）
- TaskDetailPanel：抽出 save 语义（tagNames 解析：逗号/中文逗号分隔+trim+去空、
  标题 trim、无变化跳过）
- 三处均为行为保持抽取；组件调用抽出的函数
- 新增 vitest ≥3 文件、关键分支断言 ≥15 条

## Acceptance Criteria

- [ ] `pnpm test:unit` 新增测试全绿（总数 ≥78）
- [ ] `pnpm typecheck`、`pnpm build` 不受影响
- [ ] 抽取后组件行为不变（e2e 套件后续 CI 兜底）

## Out of Scope

- jsdom/DOM 级组件测试（环境未配置，收益待评估）
- 后端测试
