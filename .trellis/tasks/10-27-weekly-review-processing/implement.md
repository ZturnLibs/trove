# 实施计划：周回顾处理模式

> 依 `design.md`。纯前端，零后端/迁移。挂点见 design 与 research。

## 0. 前置

- [x] 分支 `feat/weekly-review-processing`（已建）；阅读 `.trellis/spec/frontend/state-management.md`（失效面纪律）

## 1. 纯函数层

- [x] `processingSummary.ts`：`processingSummary(actions)` + `flattenQueues(snapshot)` 导出
- [x] vitest：summary 空/混合/全跳过；flatten 顺序与空队列跳转

## 2. ProcessingSession 组件

- [x] 队列状态机（queueIndex/actions/动作分发到四个 IPC）
- [x] 出牌卡 UI（进度、任务信息、四键 + 快捷键标注、Esc 退出）
- [x] 出牌前 status 校验（已完结自动跳过并提示）
- [x] 键盘监听（1-4/Esc，输入框聚焦短路）

## 3. 页面集成

- [x] 「开始处理」入口（全空置灰）
- [x] 结束统计卡 + 接入既有「完成回顾」summary
- [x] 失效沿用页面既有 `["tasks"]`/`["weekly-review"]`

## 4. 验证

- [x] `pnpm typecheck`、`pnpm test:unit` 全绿
- [x] `task.py validate 10-27-weekly-review-processing`
- [x] 人工矩阵：四键各一、Esc 中断保留会话、统计与动作一致、撤销 toast 可回滚

## 回滚点

- 单提交 revert 即可；无迁移、无 IPC 变更。
