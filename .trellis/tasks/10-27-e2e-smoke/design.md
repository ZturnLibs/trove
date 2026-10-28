# 设计：核心流 e2e 冒烟测试

## 1. 技术选型（依 research/tauri-driver-survey.md）

- **客户端：fantoccini（Rust, async）**——与 `cargo test` 同入口；`tokio` 已随 tauri 在依赖树中。
- **目录**：`src-tauri/tests/e2e/`（`main.rs` + `helper/mod.rs` + 按流分文件）；`#[tokio::test] #[ignore]`，环境变量 `E2E=1` 门控。
- **驱动**：测试 helper 自动 spawn `tauri-driver`（随机空闲端口），用例结束 kill；被测二进制 `tauri:options.binary` 取 `E2E_BINARY`，缺省 `target/debug/trove`（先 `cargo build`）。

## 2. 数据隔离（必须项，兼通用能力）

**新增 `TROVE_DATA_DIR` 环境变量覆盖**：DB 路径解析处（`infrastructure/db` / `lib.rs` 的 `opening database` 日志所在链路）优先读该 env。每个用例 `tempdir()` 注入——e2e 永不触碰真实用户库。附单测：env 设置后路径生效。

## 3. 用例集（10 条，2 条预期 `#[ignore]`）

| # | 用例 | 方式 |
| --- | --- | --- |
| 1 | 启动→侧栏六入口渲染 | 元素查询 |
| 2 | QuickWindow NL 建任务→收件箱命中 | 快捷键/入口点击 + 输入 |
| 3 | 完成任务→撤销 toast→回滚 | 点击 + 断言 |
| 4 | 双击改名→重开保持 | WebDriver double-click action；不可行则记 `#[ignore]` |
| 5 | 看板跨列拖拽 | `#[ignore]`（PointerSensor 不响应合成拖拽，已实测） |
| 6 | 看板同列排序 | `#[ignore]`（同上） |
| 7 | 分组新建→侧栏树 | 右键/入口点击 |
| 8 | 全局搜索命中 | QuickWindow 输入 + 结果断言 |
| 9 | URL Scheme 直达 | dev 二进制未注册 scheme → `#[ignore]`（装注册流程后启用），设计定稿时再评 |
| 10 | CSV 导出 | **走 `trove-cli`**（文件对话框 WebDriver 不友好），断言文件非空 |

> 6/9 若实现顺利可移出 ignore；交付标准以「8 条稳定绿 + 2 条有结论」为准。

## 4. CI（`.github/workflows/ci.yml` 新 job）

- `e2e`（ubuntu-latest）：装 `libwebkit2gtk-4.1-dev` + `webkit2gtk-driver`（含 `WebKitWebDriver`）+ `tauri-driver`（`cargo install tauri-driver --locked`）；`cargo build`；`E2E=1 cargo test --test e2e -- --ignored`；失败上传截图 artifact。
- 与现有 test job 并行，不计入默认 `cargo test`。

## 5. 本地体验

`E2E=1 cargo test --test e2e -- --ignored`；前提：无其他 Trove 实例（single-instance）。

## 6. 风险

- WebKitGTK 与 macOS WKWebView 行为差异 → Linux 冒烟 + macOS 本地真验证。
- fantoccini API 与当前 tauri-driver 协议版本匹配 → helper 集中封装，出问题只改一处。
