# 工作台

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
