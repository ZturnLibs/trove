# v1.4 统一动作层（切片 1）

## Goal

建立内部 `WorkbenchAction` 协议，让 URL Scheme 与未来 CLI/自动化共用同一套校验与分发逻辑。

## Requirements

1. 定义 `WorkbenchAction` / `ActionOutcome` / `ActionDispatchOptions`（domain 层）
2. `UrlSchemeAction` → `WorkbenchAction` 转换，行为与 v1.3.0 一致
3. `WorkbenchActionService` 统一处理：导航、搜索、创建预览（仍须 UI 确认）
4. 新增 IPC `workbench_action_dispatch` 供调试与未来 CLI
5. 文档：`docs/action-layer.md`

## Acceptance Criteria

- [ ] `trove://` 行为不变（导航 / 搜索 / 创建预览）
- [ ] Rust 单测覆盖 action 转换与确认门禁
- [ ] `cargo test` 全绿

## 明确不做（本切片）

- 本地 CLI 二进制
- 规则自动化引擎
- 静默 create（无确认）

## 依据

- `docs/post-v1-iteration-design.md` §8.3 统一动作层
- `docs/url-scheme.md` §与 v1.4 动作层
