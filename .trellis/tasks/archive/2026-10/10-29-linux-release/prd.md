# N1 Linux 官方发布通道

## Goal

release workflow 当前仅构建 macOS（universal）与 Windows。Linux 是 e2e 已全覆盖（10 条核心流 ubuntu 全绿）却没有官方构建产物的平台。在 release matrix 中加入 Linux job，产出 `.deb` + `.AppImage` 并纳入 updater `latest.json`。

## Requirements

- release.yml publish matrix 增加 `ubuntu-latest`（args 空、rust-targets 空）
- 构建前置依赖与 e2e job 对齐（libwebkit2gtk-4.1-dev 等），另装 `libfuse2`（AppImage 打包工具需要）
- updater（已 active + pubkey + endpoint）自动产出 Linux updater 产物并汇入 latest.json
- bundle.targets 为 "all"，无需改 tauri.conf.json

## Acceptance Criteria

- [ ] 下一次 tag 的 release workflow 出现 Linux publish job 且绿
- [ ] GitHub Release 资产含 `.deb` / `.AppImage`（+updater `.sig`）
- [ ] latest.json 含 linux x86_64 条目

## Notes

- release workflow 仅 tag 触发（无 workflow_dispatch）；本 PR 的改动在下一次发版时验证。
- workflow 头注释已预留本改动：「Linux 后续在 matrix 中加一项即可」。
