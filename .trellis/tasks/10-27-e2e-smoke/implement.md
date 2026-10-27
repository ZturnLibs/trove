# 实施计划：核心流 e2e 冒烟

> 依 `design.md`。工作于独立 worktree（与主树并行开发）。
>
> **平台注记（2026-10-27 实施修正）**：`tauri-driver` 官方仅支持 Linux/Windows——macOS 无 WKWebView WebDriver 实现。本地（macOS）验证为「编译 + 门控跳过」；真实用例执行由 CI `e2e` job（ubuntu）承担。`helper/mod.rs` 平台门控即为此设计。

## 0. 前置

- [x] 分支 `feat/e2e-smoke`（worktree 已建）；`cargo build` 确认基线可编译

## 1. 数据隔离

- [x] `TROVE_DATA_DIR` 覆盖：DB 路径解析优先读 env（`lib.rs` database open 链路）
- [x] 单测：env 注入后路径生效；未设置走原路径（`lib.rs` `#[cfg(test)]`）

## 2. e2e 基础设施

- [x] dev-deps：`fantoccini 0.22`（feature `rustls-tls`）、`tokio` full、`tempfile` 复用
- [x] `tests/e2e/helper/mod.rs`：spawn tauri-driver（随机端口 + native-port）、connect（capabilities `tauri:options.application`）、平台门控 + 全局串行锁、tempdir + env 注入、失败截图/page.html/driver 日志 artifact、`reconnect`（用例 #4 重开）、CLI 子进程转发（`--app` + 继承 `TROVE_DATA_DIR`）
- [x] `tests/e2e/main.rs`：`E2E=1` 门控 + 10 条用例
- [x] 用例 #1（启动/侧栏）跑通编译与门控路径

## 3. 用例

- [x] #2 收件箱新建、#3 完成与撤销、#4 双击改名（JS 合成 dblclick + 非受控输入置值，**可执行**，另加重开校验）、#7 分组新建、#8 任务页搜索、#10 CLI 导出（workbench action `exportTasksCsv` + `trove-cli export-csv`）
- [x] #5/#6 拖拽、#9 深链：`#[ignore]` + 结论注释（PointerSensor 不响应合成拖拽已实测；dev 二进制未注册 scheme）

## 4. CI

- [x] `ci.yml` 加 `e2e` job（ubuntu：`webkit2gtk-driver` + `cargo install tauri-driver` + build + 运行 + 失败 artifact）

## 5. 验证

- [x] 本地 macOS：`E2E=1 cargo test --test e2e -- --ignored` → 10 passed（全部经平台门控跳过，证明门控/装配/编译正确）；真实执行在 CI ubuntu
- [x] 默认 `cargo test` 不受影响（e2e `#[ignore]`，lib 199 绿）；`pnpm test:unit` 不涉及
- [ ] CI `e2e` job 绿（PR 上观察；选择器首跑可能需迭代）

## 回滚点

- 分支独立；`TROVE_DATA_DIR` 与 e2e 文件分离提交，可单独 revert。
