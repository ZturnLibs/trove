# 直接粘贴撤回能力声明（direct-paste）

## Goal

撤回「直接粘贴（directPaste）」的能力声明：该能力从未实现，但设置/剪切板 UI 宣称 macOS/Windows 可用，属「宣称与实现不符」。本次将声明改为「未提供」，并清理相关 UI，避免误导。

## 背景（勘察确认）

- `platform/mod.rs:39-49`：`direct_paste.available = cfg!(any(macos, windows))`（macOS/Windows 宣称可用），notes 提及辅助功能/系统输入。
- **全库无任何实际直接粘贴执行代码**（无 osascript/输入模拟；`clipboard_copy` 只是 write_text/write_image）。
- `ClipboardPage.tsx:251-252`：`directPasteAvailable = healthQuery.data?.capabilities.directPaste.available ?? true`。
- `ClipboardPage.tsx:282-295`：「无法直接粘贴」`PermissionBanner`（kind="accessibility"），仅在 `pasteBanner && !directPasteAvailable` 时显示（因当前 macOS/Windows available=true，该横幅基本不显示）。
- `pasteBanner` state 仅此横幅使用；`dismissBanner`/`isBannerDismissed` 被 NotificationPermissionBanner 共用（不可删全局）。

## Requirements

- R1 `platform/mod.rs`：`direct_paste.available` 恒为 `false`（撤回），notes 改为「直接粘贴暂未提供，请使用「再次复制」后手动粘贴。」（不区分平台）。
- R2 `ClipboardPage.tsx`：移除「无法直接粘贴」`PermissionBanner` 及其 `pasteBanner` state 与 `isBannerDismissed`/`dismissBanner` 的引入（仅当该文件不再使用它们时移除引入）。
- R3 保留「再次复制」按钮行为与相关文案；不影响其它能力横幅（通知权限横幅等）。

## Acceptance Criteria

- [ ] AC1 全平台 `capabilities.directPaste.available == false`，notes 文案诚实。
- [ ] AC2 剪切板页不再出现「无法直接粘贴」横幅；无残留 directPaste 引用（代码内 `rg directPaste` 仅剩类型/设置页允许，剪切板页无）。
- [ ] AC3 `pnpm typecheck`、`pnpm build`、`cargo check` 通过。

## Notes

- 轻量任务：PRD-only。改动为 Rust（platform/mod.rs）+ 前端（ClipboardPage.tsx）。
- 决策已确认：撤回声明（不实现 directPaste）。
