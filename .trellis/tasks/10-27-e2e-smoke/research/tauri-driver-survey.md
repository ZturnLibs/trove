# Research：核心流 e2e 冒烟（tauri-driver 可行性调研）

## 基线事实（2026-10-27 复核）

- **Tauri v2**（`src-tauri/Cargo.toml`：`tauri = "2"`，插件全家桶 v2）——`tauri-driver` 原生支持 v2。
- **CI**：`.github/workflows/ci.yml` 仅 2 个 job，均 `ubuntu-latest`（`:10`/`:26`，含 Linux Tauri 依赖安装 `:36`）——e2e 可先挂 Linux（免费）跑通，macOS 本地跑，Windows 缓后（见风险）。
- **单实例插件在用**：`tauri-plugin-single-instance`（deep-link feature）——WebDriver 启动被测实例时，若本机已有常驻 Trove 会冲突；e2e 环境需保证无其他实例（CI 天然满足；本地跑前需退出正式版，2026-10-26 实测踩过）。
- `withGlobalTauri` 未开启——WebDriver 协议不依赖 `window.__TAURI__`，非阻塞。

## 技术路径

- `tauri-driver`（tauri v2 自带）+ 任一 WebDriver 客户端：
  - Rust: `fantoccini`；Node: `webdriverio`。倾向 **fantoccini + Rust 集成测试**（与 `cargo test` 同一入口，`tests/e2e/` 下 `#[ignore]` 标注，显式 feature/环境变量触发）。
- 被测二进制：`tauri build --debug` 产物或 `cargo build` 后由 `tauri-driver` 拉起（dev server 模式需 vite 并行，倾向**构建产物**，避免 CI 双进程编排）。
- 元素定位依赖 WebView 可访问性树——Tauri v2 WKWebView/WebView2/WebKitGTK 均可；Linux 需 `WebKitWebDriver` 包（CI `Install Linux Tauri deps` 步骤需追加）。

## 候选冒烟用例（10 条，对应 PRD R2）

1. 启动→主窗口出现→侧栏六入口渲染
2. 快速捕获建任务（NL 输入）→ 出现在收件箱
3. 任务完成 → 今日页完成区出现 → 撤销 toast 回滚
4. 任务双击改名 → 刷新保持
5. 看板：拖卡（跨列 doing）→ stage 持久化（重启校验）
6. 看板：同清单列内排序 → 刷新保持
7. 分组：新建分组/清单 → 侧栏树出现
8. 全局搜索（QuickWindow）四类数据命中
9. URL Scheme `trove://today` 直达
10. CSV 导出文件生成非空

## 风险与开放问题

| 风险 | 应对 |
| --- | --- |
| Linux WebKitGTK 行为与 macOS WKWebView 有差异 | CI 只当冒烟网；macOS 本地为真实验证 |
| Windows WebView2 需 `--webview2-strategy` 特殊 flag | v1 不做 Windows CI e2e，本地手动 |
| 拖拽（dnd-kit PointerSensor distance 8）在 WebDriver 合成事件下可能不触发 | 优先验证点击/键盘流；拖拽用例标记 `#[ignore]` 待定（orca 合成拖拽已实测无法激活 sensor） |
| 深链用例依赖 single-instance + deep-link 注册 | 用例 9 需应用已安装注册或 `--deep-link` 直启参数，设计阶段定 |
| CI 时长 | e2e job 与现有 test job 并行；构建 debug 产物约 3-5min 可接受 |

## 待设计阶段决策

- WebDriver 客户端选型（fantoccini vs webdriverio）与目录布局（`src-tauri/tests/e2e/` vs 顶层 `e2e/`）。
- 断言粒度：仅「流程可达」还是含数据校验（读 DB / CLI 输出）。
