# Logo 设计与全量更新

## Goal

为 Trove（本地优先的个人工作台）设计全新 Logo（意象：宝藏箱 + 字母 T，品牌蓝单色），并更新所有使用 Logo 的位置：应用图标资产（icns/ico/多尺寸 PNG）、favicon、侧边栏品牌标、Onboarding 弹层、托盘图标、默认资源清理。

用户价值：建立与产品气质一致（宝藏库、收纳、简洁）的视觉身份，替换 Tauri/Vite 默认图标，使各平台（Dock/任务栏/托盘/窗口/浏览器标签）呈现统一的品牌形象。

## 背景（代码勘察确认）

- 品牌色：主题系统蓝 `#2563eb`（亮）/ `#5b8def`（暗），见 `src/styles/theme.css`。
- 现有图标资产 `src-tauri/icons/`（Tauri 默认图标）：`32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.png`、`icon.icns`、`icon.ico`、`Square30/44/71/89/107/142/150/284/310x310Logo.png`、`StoreLogo.png`。`tauri.conf.json` `bundle.icon` 引用其中 5 个。
- 托盘图标：`src-tauri/src/lib.rs:87` 用 `app.default_window_icon()`（随 bundle 图标自动更新，无需改 Rust）。
- `public/`：`vite.svg`（index.html favicon 引用 `/vite.svg`）、`tauri.svg`（Tauri 默认 logo，未在代码引用）。
- 应用内品牌：`MainShell.tsx:78` 侧边栏纯文本 "Trove"；`OnboardingOverlay.tsx:29` "欢迎使用 Trove"。
- 窗口/产品名：`productName: "Trove"`、主窗 `title: "Trove"`（保持不变）。
- 图标再生成工具：`pnpm tauri icon <source>`（Tauri CLI 内置），接受 1024×1024 方形透明 PNG/SVG，输出到 `src-tauri/icons/`。

## Requirements

- R1 设计 Logo：宝藏箱 + 字母 T 意象，品牌蓝单色（主题蓝 `#2563eb`，暗色可用 `#5b8def`），简洁几何、小尺寸可辨。
- R2 用 `tauri icon` 从源图再生成 `src-tauri/icons/` 全平台图标集（含 icns/ico 及各尺寸 PNG），覆盖 `bundle.icon` 引用与托盘/窗口图标。
- R3 提供矢量 SVG 作为 favicon（`public/`），更新 `index.html` 引用；清理不再使用的默认资源（`vite.svg`、`tauri.svg`）。
- R4 侧边栏（`MainShell.tsx`）显示 Logo 标 + "Trove" 字标，随主题自适应（亮/暗）。
- R5 Onboarding 弹层（`OnboardingOverlay.tsx`）展示 Logo。
- R6 保持 `productName`/窗口标题 "Trove" 不变，不破坏现有布局与功能。

## Acceptance Criteria

- [ ] AC1 产出 Logo 概念图（宝藏箱 + T，蓝色单色），视觉与现有蓝色主题一致。
- [ ] AC2 `pnpm tauri icon` 后 `src-tauri/icons/` 全部图标更新为新 Logo；`tauri build` 产物（macOS icns/Windows ico）显示新图标。
- [ ] AC3 favicon 更新为新 Logo，`index.html` 无失效资源引用；默认 `vite.svg`/`tauri.svg` 已清理或不再引用。
- [ ] AC4 侧边栏展示 Logo 标 + "Trove"，亮/暗主题下均清晰可见。
- [ ] AC5 Onboarding 弹层展示新 Logo。
- [ ] AC6 托盘图标（Dock/菜单栏）显示新 Logo（经 `default_window_icon`）。
- [ ] AC7 `pnpm typecheck`、`pnpm build` 通过；无回归。

## Out of Scope

- 重新设计产品名/文案。
- 官网、营销素材、iOS/Android 移动端图标（当前无移动端）。
- 动态/动效 Logo。

## Key Decisions（已与用户确认）

- 意象：宝藏箱 + 字母 T。
- 色彩：品牌蓝单色（跟随主题蓝，暗色适配）。
- 工具链：`tauri icon` 统一再生成图标集；应用内 Logo 用内联 SVG 组件（`currentColor`）实现主题自适应。
