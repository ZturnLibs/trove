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
- [x] CI 挂死修复（run 40489217286 诊断）：ubuntu-latest 无显示服务器，WebKitGTK app 开不了窗 → WebKitWebDriver 会话创建永不返回。修复 = 官方方案：装 `xvfb` + `xvfb-run -a` 包裹；`timeout-minutes` 兜底（超时属 failure，artifact 仍上传）；helper `connect()` 加 `CONNECT_TIMEOUT`（此前裸 await 曾挂满 6h job 上限）、失败截图/取源码加 5s 限时、`run_cli` 加 30s 超时强杀
- [x] CI 握手超时修复（run 40703046029 诊断，artifact tauri-driver.log：`org.a11y.Bus ServiceUnknown` 后 `IncompleteMessage`）：xvfb 后 app 已起，但 CI 无 session DBus → WebKitGTK 自动化依赖的 AT-SPI 可访问性总线解析失败，握手挂到超时。修复 = `dbus-run-session -- xvfb-run -a` 包裹 + 显式装 `dbus`/`at-spi2-core`；`WEBKIT_DISABLE_DMABUF_RENDERER=1`（无 GPU 软渲染兜底）；`fc-cache -f` 预热 + `CONNECT_TIMEOUT` 30→90s（首启冷建缓存）
- [x] CI 白窗修复（run 40727225974 诊断）：会话已建（全部用例「已连接」），但 7 条同败于「等待侧栏（今日）超时」——`cargo build` 的 debug 二进制加载 `devUrl`（CI 无 vite server）→ 白窗。修复 = 按设计文档改 `cargo tauri build --debug --no-bundle`（内嵌 `frontendDist`），job 补 pnpm/node setup + `tauri-cli` 安装
- [x] CI 事件管线旁路：2026-10-28 14:10Z 起 GitHub 对本仓库 push/PR 事件不再产生 run（rerun 正常，19h 未愈）→ ci.yml 加 `workflow_dispatch`，`gh workflow run ci --ref feat/e2e-smoke` API 直发验证

## 5. 验证

- [x] 本地 macOS：`E2E=1 cargo test --test e2e -- --ignored` → 10 passed（全部经平台门控跳过，证明门控/装配/编译正确）；真实执行在 CI ubuntu
- [x] 默认 `cargo test` 不受影响（e2e `#[ignore]`，lib 199 绿）；`pnpm test:unit` 63 绿
- [ ] CI `e2e` job 绿（PR 上观察；选择器首跑可能需迭代）

## 回滚点

- 分支独立；`TROVE_DATA_DIR` 与 e2e 文件分离提交，可单独 revert。
