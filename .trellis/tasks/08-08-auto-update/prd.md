# Trove 自动更新

## Goal

为 Trove 桌面客户端接入基于 GitHub Releases 的自动更新能力：发布新版本后，已安装用户可收到更新提示并完成签名验证后的下载与安装，且不触碰本地数据库。

## 背景（勘察确认）

- 发版：`scripts/release.sh` bump 版本 → push `main` + `v*` tag → `.github/workflows/release.yml` 用 `tauri-action` 构建 macOS universal + Windows 并发布 GitHub Release。
- CI 已预留 `TAURI_PRIVATE_KEY` 注释，尚未配置 updater 插件与 `createUpdaterArtifacts`。
- `docs/technical-design.md` 约定：Tauri Updater 签名更新；更新失败不修改用户数据库。
- 当前客户端无 `tauri-plugin-updater`；关于/设置页仅展示版本号。

## Requirements

- R1 接入 Tauri 2 官方 `updater` + `process` 插件，配置 GitHub Releases 静态 manifest endpoint（`latest.json`）。
- R2 构建侧开启 `createUpdaterArtifacts`，CI 注入签名私钥，Release 资产含更新包、`.sig` 与 `latest.json`。
- R3 客户端支持：手动检查更新、启动后延迟自动检查（可设置关闭）、下载进度、安装后重启。
- R4 macOS universal 构建使用 `macos-universal` target 检查更新。
- R5 Windows 安装前触发清理钩子（与现有退出逻辑一致），`installMode: passive`。
- R6 开发模式跳过更新检查；更新流程不读写 `workbench.db`。
- R7 设置页与关于对话框展示更新状态并提供操作入口。

## Acceptance Criteria

- [ ] AC1 `tauri.conf.json` 含 updater 配置与 `createUpdaterArtifacts: true`；release workflow 使用 `TAURI_SIGNING_PRIVATE_KEY`。
- [ ] AC2 设置页可开关「自动检查更新」、手动「检查更新」；关于页显示当前版本与更新状态。
- [ ] AC3 启动约 30s 后（且开启自动检查时）后台静默检查，有更新时非阻塞 toast 提示。
- [ ] AC4 `pnpm typecheck`、`cargo test`、`cargo clippy` 通过。
- [ ] AC5 私钥不进仓库；`.tauri/*.key` 已 gitignore；公钥写入 `tauri.conf.json`。

## Out of Scope

- Linux 平台构建与更新。
- Beta/预发布通道。
- 静默强制安装（须用户确认后安装）。

## Notes

- 首个带 updater 的版本发布后，此前手动安装的旧版本需再手动升级一次才能启用自动更新。
- GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 需运维在仓库配置（内容为私钥或路径说明见 implement.md）。
