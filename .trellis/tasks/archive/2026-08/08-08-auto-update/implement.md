# 实施计划：Trove 自动更新

## Checklist

- [x] 1. `.gitignore` 排除 `.tauri/`
- [x] 2. `Cargo.toml` / `package.json` 添加 updater、process 插件
- [x] 3. `tauri.conf.json` updater + `createUpdaterArtifacts`
- [x] 4. `lib.rs` 注册插件；`AppSettings.auto_check_updates`
- [x] 5. `capabilities/main.json` 权限
- [x] 6. `.github/workflows/release.yml` 签名 env
- [x] 7. `src/ipc/client.ts` 类型同步
- [x] 8. `src/stores/app-updater.ts` + `UpdateToast.tsx`
- [x] 9. `SettingsPage` / `AboutDialog` / `MainShell` 集成
- [x] 10. 验证：`pnpm typecheck`、`cargo test`；E2E Toast 检测 1.2.3；v1.2.4 Release + verify 脚本

## 运维（已完成）

- [x] GitHub Secret `TAURI_SIGNING_PRIVATE_KEY`
- [x] 仓库 Public
- [x] `docs/auto-update.md` + `scripts/verify-updater-endpoint.sh`

## 交付版本

| Tag | 说明 |
|-----|------|
| v1.2.2 | 首个含 updater 的 release |
| v1.2.3 | 修复 macOS arch target |
| v1.2.4 | 公开仓库文档 + verify 脚本 |
