# 实施计划：核心流 e2e 冒烟

> 依 `design.md`。工作于独立 worktree（与主树并行开发）。

## 0. 前置

- [ ] 分支 `feat/e2e-smoke`（worktree 已建）；`cargo build` 确认基线可编译

## 1. 数据隔离

- [ ] `TROVE_DATA_DIR` 覆盖：DB 路径解析优先读 env（`lib.rs` database open 链路）
- [ ] 单测：env 注入后路径生效；未设置走原路径

## 2. e2e 基础设施

- [ ] dev-deps：`fantoccini`、`tokio`（features full）、`tempdir` 复用
- [ ] `tests/e2e/helper/mod.rs`：spawn tauri-driver（随机端口）、connect（capabilities `tauri:options.binary` + `TROVE_DATA_DIR`）、用例结束清理 + 失败截图
- [ ] `E2E=1` 门控骨架 + 首条用例 #1（启动/侧栏）跑通本地 macOS

## 3. 用例

- [ ] #2 NL 捕获、#3 完成与撤销、#4 双击改名（不可行则 ignore 记录）、#7 分组新建、#8 全局搜索、#10 CLI 导出
- [ ] #5/#6 拖拽、#9 深链：`#[ignore]` + 结论注释（可启用条件写明）

## 4. CI

- [ ] `ci.yml` 加 `e2e` job（ubuntu：依赖 + tauri-driver + build + 运行 + artifact）

## 5. 验证

- [ ] 本地 macOS：`E2E=1 cargo test --test e2e -- --ignored` 稳定绿（非 ignore 用例）
- [ ] 默认 `cargo test` 不受影响；`pnpm test:unit` 不受影响
- [ ] `task.py validate 10-27-e2e-smoke`

## 回滚点

- 分支独立；`TROVE_DATA_DIR` 与 e2e 文件分离提交，可单独 revert。
