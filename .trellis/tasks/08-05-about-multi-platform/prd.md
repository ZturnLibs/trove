# About 跨平台（about-multi-platform）

## Goal

为 Windows/Linux（以及 macOS 帮助菜单）提供「关于 Trove」入口，打开应用内 About 弹窗，展示品牌 Logo、版本、标语与版权，与 macOS 原生 About 信息一致。

## 背景（勘察确认）

- macOS：应用菜单含 `PredefinedMenuItem::about`（menu_bar.rs:306，含版本/标语/版权元数据）。
- 非 macOS 菜单（menu_bar.rs:350-361）：无任何 About 入口。
- 帮助菜单 `help_menu`（menu_bar.rs:293-302）：含「快捷键一览」「隐私与数据说明」，均 navigate 到 /settings（menu_bar.rs:398-400）。
- 前端已有 `BrandLogo`（currentColor）、`appHealth.appVersion`（ipc.appHealth）。弹窗样式参照 OnboardingOverlay / AttachmentsSection 遮罩面板。

## Requirements

- R1 帮助菜单新增「关于 Trove」（id `menu.help.about`），macOS 与 Windows/Linux 均添加。
- R2 `handle_menu_event` 对 `menu.help.about` 分发 `menu://about` 事件到主窗口（沿用 `main://navigate` 的 emit 模式）。
- R3 新增 `src/components/AboutDialog.tsx`：遮罩弹窗，含 BrandLogo、产品名「Trove」、版本（`appHealth.appVersion`）、标语「本地优先的个人工作台」、版权「© 2026 Trove」、以及「打开设置」按钮。
- R4 `MainShell` 监听 `menu://about`，打开 AboutDialog；弹窗支持遮罩点击/「关闭」关闭。
- R5 macOS 原生 About 保留不变。

## Acceptance Criteria

- [ ] AC1 Windows/Linux 帮助菜单出现「关于 Trove」，点击打开 About 弹窗。
- [ ] AC2 弹窗展示 Logo、产品名、版本（来自 appHealth）、标语、版权。
- [ ] AC3 弹窗可关闭；macOS 原生 About 不受影响。
- [ ] AC4 `cargo check`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 中复杂度任务：PRD + 简要 design。改动为 menu_bar.rs（小）+ 前端（AboutDialog、MainShell）。
- 不新增 IPC 命令：版本用既有 `appHealth`，事件用既有 `app.emit` + 前端 `listen`。
