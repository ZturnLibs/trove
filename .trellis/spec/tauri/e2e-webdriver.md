# E2E（tauri-driver / WebKitGTK）契约

`src-tauri/tests/e2e/` + CI `e2e` job 的可执行契约。改动任一侧前必读。

## Scenario: tauri-driver e2e 冒烟（Linux CI）

### 1. Scope / Trigger

- 触发：改 `tests/e2e/**`、ci.yml 的 `e2e` job、app 启动 env/data-dir 链路、或新增用例。
- 性质：Infra integration（WebDriver + 显示/DBus + feature 门控）→ 本文件为强制契约。

### 2. Signatures

- 运行入口（本地 Linux/CI 同形）：

```bash
dbus-run-session -- xvfb-run -a cargo test --test e2e --features custom-protocol -- --ignored --nocapture
```

- Harness 公开面（`tests/e2e/helper/mod.rs`）：
  - `Harness::acquire(case)` → 串行锁 + tempdir + `TROVE_DATA_DIR` 注入 + spawn tauri-driver + connect
  - `Harness::reconnect()` → 显式 `close()` 旧会话（clone 句柄）→ 等 app 退出 → 新会话（同 tempdir）
  - `Ctx::click/wait_for/wait_gone/find`、`Ctx::double_click_retry`（W3C Actions 真实指针）、
    `Ctx::arm_editor_submit` / `Ctx::editor_submit_status`（页内驻留监听）、`Ctx::run_cli`（30s 上限）
- env：`E2E=1`（门控，缺省跳过）、`E2E_TAURI_DRIVER`、`E2E_BINARY`、`E2E_CLI`、
  `E2E_ARTIFACT_DIR`（CI artifact 根）、`TROVE_DATA_DIR`（数据隔离，硬约束）

### 3. Contracts

- **feature 一致**：`custom-protocol` 必须同时出现在 `cargo build` 与 `cargo test` 两步。
  cargo test 会重编 bin；feature 集不一致 → 指纹不匹配重编 → debug 二进制退回 devUrl 模式。
- **会话上限**：WebKitWebDriver 每驱动实例单会话。旧 `Client` 必须 `close()`（drop 静默不关）。
- **单实例**：e2e 机器不得有其他 Trove 实例（single-instance 插件约束）；CLI 转发依赖运行中的 app。
- **数据隔离**：每用例独立 tempdir，`TROVE_DATA_DIR` 指向它；用例结束逆序清理。
- **留证**：失败 → error.txt + failure.png + page.html + tauri-driver.log + tasks-on-failure.csv（CLI DB 快照）。

### 4. Validation & Error Matrix

| 症状 | 根因 | 对策 |
|---|---|---|
| 用例挂满 job 上限（无输出） | 无 X：WebKitGTK 开不了窗，会话创建永不返回 | `xvfb-run -a` 包裹 + 装 `xvfb`；job `timeout-minutes` 兜底 |
| 「连接 tauri-driver 超时」+ log 有 `org.a11y.Bus ServiceUnknown` | 无 session DBus，AT-SPI 握手挂起 | `dbus-run-session --` 包裹 + 装 `dbus`/`at-spi2-core` |
| 白窗 / page.html = `Could not connect to localhost` | 未开 `custom-protocol`，debug 二进制走 devUrl | build/test 两步统一 `--features custom-protocol` |
| `Maximum number of active sessions` | 旧会话未关（Client drop 不关） | reconnect 先 `clone().close()` |
| 「找不到 X: webdriver returned error: <空>」 | find 一次性、元素在视图切换渲染窗口期未挂载 | 用 `wait_for`（20s 重试）替代裸 `find` |
| `Node with identifier ... was not found` | React 重渲染使节点脱钩（stale） | click 整体重试；或页内 JS 原子操作 |

### 5. Good/Base/Bad Cases

- Good：`E2E=1` + 上述包裹命令 → 10 条用例 ~41s 全绿（run 40753626139 基线）。
- Base：macOS 本地 `E2E=1 ... --ignored` → 10 passed 全部跳过（平台门控路径）。
- Bad：`cargo build` + 裸 `cargo test --test e2e`（无 xvfb/dbus/feature）→ 6h 挂死或白窗。

### 6. Tests Required

- 新用例必须：`#[tokio::test] #[ignore]` + `Harness::acquire` 门控；断言用 `wait_for`（禁裸 find）。
- 完成类操作断言「行离开 todo 视图」而非行内按钮翻转（inbox 过滤 todo）。
- 撤销 toast 仅详情面板完成路径产生（行内完成静默）。
- 默认 `cargo test --lib` 与 `pnpm test:unit` 不受影响（e2e 全部 `#[ignore]`）。

### 7. Wrong vs Correct

#### Wrong

```rust
// JS 合成 dblclick——WebKitGTK 上不触发 React onDoubleClick（trusted/delegate 差异）
el.dispatch_event & new MouseEvent('dblclick')   // 多轮实证：编辑框永不出现
// 以及编辑框出现后跨命令 send_keys——节点毫秒级被替换，句柄必脱钩
input.send_keys(...).await                        // 首键即 stale
```

#### Correct

```rust
// 1) 真实指针：W3C Actions（MoveTo 元素中心 + Down/Up×2）
// 2) 提交：arm_editor_submit 预埋页内驻留监听（30ms 轮询），编辑框出现
//    立即「原型 value setter + input 事件 + Enter」原子完成——无跨命令句柄
```

### 常见坑（速查）

- **重开校验**必须复用同一 Harness（`reconnect` 同 tempdir）；重新 `acquire` 开新 tempdir
  → 空库 → 持久化断言永假（tasks-on-failure.csv 可实证）。
- **编辑态定位**：进入编辑后行的 text content 不再含标题（标题 div 被 input 替换），
  `contains(., title)` 类定位自败——用编辑框自身特征（`input.h-7`）。
- **点击重试**：WebKitGTK 偶发空错误消息的 stale/会话抖动，文案匹配不可靠——
  click 失败一律整体重试（重新 wait_for 再点）。
