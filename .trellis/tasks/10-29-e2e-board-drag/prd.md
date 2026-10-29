# 启用看板拖拽 e2e 用例（case05/06）

## Goal

`10-27-e2e-smoke` 归档时 case05（跨列）/case06（同列排序）因「PointerSensor 不响应合成拖拽」`#[ignore]`，启用条件为「WebDriver 原生 action 序列验证后启用」。该条件已满足：case04 双击改名实证 W3C Actions 真实指针序列（`native_double_click`）在本栈可用。本轮将拖拽改为真实指针序列并启用两条用例。

## Requirements

- helper 新增 `native_drag`（MoveTo 源中心 → Down → 小位移越 8px 激活阈值 → MoveTo 目标 → Up，全 W3C Actions）
- case05：看板新建「e2e 跨列」→ 拖卡到「进行中」列 → 断言卡片进入 doing 列
- case06：新建「e2e 排序甲/乙」→ 拖甲到乙上 → 断言待办列内顺序互换
- 移除两条用例的 `#[ignore]`（改标准门控 ignore 理由）
- CI 全绿；默认 cargo test / pnpm test:unit 不受影响

## Acceptance Criteria

- [ ] CI `e2e` job 绿且包含 case05/06（10 → 全部通过，无 ignore 跳过除 case09 深链）
- [ ] 失败时按既有留证链可诊断（截图/DB 快照）

## Out of Scope

- case09 deep-link（需打包注册 scheme，维持 ignore）
- Windows/macOS CI e2e

## Notes

- 若真实指针拖拽仍无法驱动 dnd-kit（PointerSensor），按 PRD 惯例回退 `#[ignore]` 并更新结论注释。
