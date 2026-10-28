# 核心流 e2e 冒烟测试

## Goal

建立端到端回归网：用 tauri-driver（WebDriver）驱动**真实构建产物**，覆盖 10 条核心用户流，进 CI（Linux 先行）。当前 197 cargo + 63 vitest 全是单测，拖拽/乐观更新/深链类缺陷（2026-10-26 实测两例）只有 e2e 能兜住。

## Background（证据锚点）

- Tauri v2 + `tauri-driver` 兼容；CI 现状与风险：`research/tauri-driver-survey.md`。
- 单实例插件是本地运行的前置约束（CI 无此问题）。

## Requirements

### R1 基础设施

- e2e 独立入口（建议 `src-tauri/tests/e2e/` + `#[ignore]`，环境变量 `E2E=1` 触发），不拖慢默认 `cargo test`。
- `tauri-driver` 拉起 debug 构建产物；用例间重置状态（临时 data dir 或专测 profile）。
- CI：新增 e2e job（ubuntu-latest），与现有 test 并行；安装 WebKitWebDriver 依赖。

### R2 冒烟用例集（10 条）

启动渲染 / NL 快速捕获 / 完成与撤销 / 双击改名 / 看板跨列 / 看板同列排序 / 分组新建 / 全局搜索 / URL Scheme 直达 / CSV 导出——清单与逐条前置见 `research/tauri-driver-survey.md`。

### R3 失败可诊断

- 失败时保留截图与 WebDriver 日志为 CI artifact；用例输出带步骤名。

## Acceptance Criteria

- [ ] CI 新增 e2e job 全绿；本地 `E2E=1 cargo test --test e2e -- --ignored` 可复跑
- [ ] 10 条用例覆盖上述流；拖拽类用例如确认无法激活 PointerSensor，标注 `#[ignore]` 并记录结论
- [ ] 失败用例产出截图 artifact
- [ ] 默认 `cargo test`、`pnpm test:unit` 不受影响（无 e2e 依赖泄漏）

## Out of Scope

- Windows/macOS CI e2e（Linux 冒烟 + 本地手动过渡）
- 视觉回归（截图 diff）
- 性能压测（万级负载基线另立项）

## Open Questions（设计阶段决策，见 research）

- WebDriver 客户端选型与目录布局
- 断言粒度（流程可达 vs 数据校验）
- 深链用例的注册方式

## Notes

- 复杂任务：进入实现前需补 `design.md` + `implement.md`（本 PRD 为种子）。
