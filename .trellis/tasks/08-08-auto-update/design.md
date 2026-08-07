# 技术设计：Trove 自动更新

## 1. 架构概览

```
release.sh → v* tag → GitHub Actions (tauri-action)
                              ↓
                    DMG/NSIS + .tar.gz/.exe + .sig + latest.json
                              ↓
客户端 updater plugin ← GET latest.json ← GitHub Releases
         ↓ 验签 + 版本比较
    下载 → 安装 → relaunch (process plugin)
```

Endpoint：

```
https://github.com/ZturnLibs/trove/releases/latest/download/latest.json
```

## 2. 构建与 CI

### 2.1 `tauri.conf.json`

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<minisign 公钥全文>",
      "endpoints": [
        "https://github.com/ZturnLibs/trove/releases/latest/download/latest.json"
      ],
      "windows": { "installMode": "passive" }
    }
  }
}
```

### 2.2 `release.yml`

- 环境变量改为 Tauri 2 标准：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（可选）。
- `tauri-action` 默认 `uploadUpdaterJson: true`，无需额外步骤。

### 2.3 密钥

- 本地生成：`pnpm tauri signer generate -w .tauri/trove.key --ci -p ''`
- 公钥 → `tauri.conf.json`；私钥 → GitHub Secret + 安全备份。
- `.gitignore` 排除 `.tauri/*.key`（不含 `.pub`）。

## 3. Rust 层

### 3.1 依赖

- `tauri-plugin-updater`
- `tauri-plugin-process`

### 3.2 插件注册（`lib.rs`）

```rust
.plugin(tauri_plugin_updater::Builder::new().build())
.plugin(tauri_plugin_process::init())
```

Windows `on_before_exit`：在 updater builder 中 flush 数据库连接（通过 AppState 若可及）或记录日志；Tauri 会在安装前退出应用。

### 3.3 设置扩展

`AppSettings` 新增：

```rust
#[serde(default = "default_true")]
pub auto_check_updates: bool,
```

## 4. 前端层

### 4.1 依赖

- `@tauri-apps/plugin-updater`
- `@tauri-apps/plugin-process`

### 4.2 `useAppUpdater` hook

状态：`idle | checking | upToDate | available | downloading | ready | error`

- `checkForUpdates()`：dev 模式 no-op；macOS 传 `{ target: 'macos-universal' }`。
- `downloadAndInstall()`：`update.downloadAndInstall(onProgress)` → `relaunch()`。
- 持久化 `lastCheckedAt` 于 `localStorage`（24h 节流由 hook 内实现）。

### 4.3 UI 挂载点

| 位置 | 行为 |
|------|------|
| `MainShell` | 启动 30s 后自动检查（尊重 `autoCheckUpdates`） |
| `SettingsPage` | 开关 + 手动检查 + 状态/进度 |
| `AboutDialog` | 版本行 + 「检查更新」 |
| `UpdateToast`（新建小组件） | 有更新时 toast 操作 |

### 4.4 Capabilities

`main.json` 增加：

```json
"updater:default",
"process:allow-restart"
```

## 5. 安全与数据隔离

- HTTPS endpoint；minisign 公钥编译进二进制。
- 更新仅替换应用 bundle，不调用 migration 或 backup restore。
- 不做降级（默认版本比较）。

## 6. 测试策略

- Rust：`AppSettings` 新字段 serde 默认值单测（可选，沿用 settings 测试模式）。
- 前端：`useAppUpdater` 在 non-Tauri 环境 graceful no-op（与现有 `isTauriRuntime` 模式一致）。
- CI：现有 typecheck + cargo test；不跑真实 GitHub 更新 E2E。
