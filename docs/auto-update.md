# 自动更新

Trove 使用 [Tauri Updater](https://v2.tauri.app/plugin/updater/) 从 GitHub Releases 拉取签名更新包。Release 由 `v*` 标签触发，CI 自动生成 `latest.json` 与 `.sig` 文件。

## 一次性：签名密钥

### 1. 生成密钥对（仅首次）

```bash
pnpm tauri signer generate -w .tauri/trove.key --ci -p ''
```

产出：

| 文件 | 用途 | 是否进仓库 |
| --- | --- | --- |
| `.tauri/trove.key` | 签名私钥 | **否**（已在 `.gitignore`） |
| `.tauri/trove.key.pub` | 验签公钥 | 否（公钥已写入 `src-tauri/tauri.conf.json`） |

丢失私钥后，**已安装且启用了 updater 的用户将无法再收到更新**。请把私钥备份到密码管理器或团队密钥库。

### 2. 配置 GitHub Secret

在仓库 [Settings → Secrets and variables → Actions](https://github.com/ZturnLibs/trove/settings/secrets/actions) 添加：

| Secret | 值 |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | `.tauri/trove.key` 文件的**完整内容**（含 `untrusted comment:` 行） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；无密码时可留空或不配置 |

命令行配置（需已 `gh auth login`）：

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo ZturnLibs/trove < .tauri/trove.key
```

公钥必须与 `src-tauri/tauri.conf.json` 中 `plugins.updater.pubkey` 一致。轮换密钥时需同时更新配置文件并重新发版。

## 发版流程

与往常一样在 **main** 分支执行：

```bash
./scripts/release.sh patch   # 或 minor / major / 具体版本号
```

脚本会 bump 版本、提交、打 `v*` 标签并推送。GitHub Actions `release` 工作流将：

1. 校验 tag 与 `package.json` / `tauri.conf.json` / `Cargo.toml` 版本一致
2. 构建 macOS universal 与 Windows 安装包
3. 生成 updater 包（`.app.tar.gz` / NSIS `.exe`）及 `.sig`
4. 上传 `latest.json` 到 Release assets

查看进度：<https://github.com/ZturnLibs/trove/actions>

## 验证 Release 资产

每次发版后确认 Release 页面包含：

- [ ] `latest.json`
- [ ] macOS：`*.app.tar.gz` 与 `*.app.tar.gz.sig`（以及 DMG）
- [ ] Windows：`*.exe` 与 `*.exe.sig`（NSIS 安装包）

`latest.json` 示例结构：

```json
{
  "version": "1.2.2",
  "notes": "...",
  "pub_date": "2026-08-07T...",
  "platforms": {
    "darwin-aarch64": { "signature": "...", "url": "..." },
    "darwin-x86_64": { "signature": "...", "url": "..." },
    "windows-x86_64": { "signature": "...", "url": "..." }
  }
}
```

macOS universal 构建时，客户端检查更新需使用 `target: 'macos-universal'`（已在 `src/stores/app-updater.ts` 实现）。

手动验证 manifest：

```bash
curl -fsSL https://github.com/ZturnLibs/trove/releases/latest/download/latest.json | jq .
```

## 客户端行为

| 时机 | 行为 |
| --- | --- |
| 启动约 30s | 后台检查（设置 →「自动检查更新」可关闭） |
| 每 24h | 最多自动检查一次 |
| 设置 / 关于 | 手动「检查更新」 |
| 开发模式 | 跳过检查 |

有更新时显示 toast；用户确认后下载、验签、安装并重启。更新**仅替换应用二进制**，不修改 `workbench.db`。

## 首个带 updater 的版本

在 auto-update 功能合并并发布之前已安装的用户，需要**手动安装一次**首个带 updater 的版本，之后才能自动更新。

## 本地验证 updater 产物

不推送 tag 也可在本地确认签名与 bundle 生成：

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat .tauri/trove.key)"
# 可选：export TAURI_SIGNING_PRIVATE_KEY_PASSWORD='...'
pnpm tauri build
```

检查 `src-tauri/target/release/bundle/` 下是否出现 `.tar.gz.sig`（macOS）或 `.exe.sig`（Windows）。

## 故障排查

| 现象 | 可能原因 |
| --- | --- |
| Release 无 `latest.json` | Secret 未配置或 `createUpdaterArtifacts` 未开启 |
| 客户端「检查更新失败」 | 网络、Release 尚未发布完、或 endpoint URL 错误 |
| 下载后安装失败 | 签名不匹配（公钥与 CI 私钥不配对） |
| macOS 有更新但检测不到 | universal 包需 `macos-universal` target |

## 相关文件

- `src-tauri/tauri.conf.json` — updater endpoint 与公钥
- `.github/workflows/release.yml` — CI 构建与 Release
- `scripts/release.sh` — 版本 bump 与 tag 推送
- `src/stores/app-updater.ts` — 客户端更新逻辑
