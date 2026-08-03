# Trove

一站式个人工作台桌面应用（Tauri 2 + React + SQLite）。

产品与技术设计见 [`docs/`](./docs/README.md)。当前实现对应开发规划 **阶段 0：应用基础**（`0.1.0`）。

## 开发

```bash
pnpm install
pnpm tauri:dev
```

仅前端预览（无 Rust IPC）：

```bash
pnpm dev
```

## 常用脚本

| 命令 | 说明 |
| --- | --- |
| `pnpm tauri:dev` | 启动桌面开发态 |
| `pnpm tauri:build` | 打包 |
| `pnpm typecheck` | TypeScript 检查 |
| `pnpm test:unit` | 前端单测（逐步补充） |

Rust 数据层测试：

```bash
cd src-tauri && cargo test
```

## 发布新版本

CI（`.github/workflows/release.yml`）在推送 `v*` 标签时构建 macOS（通用二进制）与 Windows 安装包，并汇总为一个 GitHub **Draft Release**，核对后再发布。

1. 同步更新 `package.json` 与 `src-tauri/tauri.conf.json` 的 `version`；
2. 提交后打标签并推送：

   ```bash
   git tag v1.2.0
   git push origin v1.2.0
   ```

3. Actions 跑完后到 [Releases](../../releases) 页发布草稿。

> 标签必须与应用版本一致，否则 CI 在 `check-tag` 阶段失败。当前 macOS / Windows 为未签名构建，首次打开会有系统提示；签名与自动更新可后续接入。
