# 实施计划：Trove 自动更新

## Checklist

- [x] 1. `.gitignore` 排除 `.tauri/*.key`
- [x] 2. `Cargo.toml` / `package.json` 添加 updater、process 插件
- [x] 3. `tauri.conf.json` updater + `createUpdaterArtifacts`
- [x] 4. `lib.rs` 注册插件；`AppSettings.auto_check_updates`
- [x] 5. `capabilities/main.json` 权限
- [x] 6. `.github/workflows/release.yml` 签名 env
- [x] 7. `src/ipc/client.ts` 类型同步
- [x] 8. `src/stores/app-updater.ts` + `UpdateToast.tsx`
- [x] 9. `SettingsPage` / `AboutDialog` / `MainShell` 集成
- [x] 10. 验证：`pnpm typecheck`、`cargo test`（settings）；clippy 既有告警未改

## 运维（发版前一次性）

1. 在 GitHub 仓库 Settings → Secrets 添加 `TAURI_SIGNING_PRIVATE_KEY`（私钥文件全文或 CI 可读内容）。
2. 若私钥有密码，添加 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
3. 发首个带 updater 的 release 后，在 Release 资产中确认存在 `latest.json` 与 `.sig` 文件。

## Rollback

- 关闭 `auto_check_updates` 默认值不影响已发布客户端；紧急时可发 hotfix 移除 endpoint 或禁用 CI 签名（旧客户端检查失败即保持当前版本）。
