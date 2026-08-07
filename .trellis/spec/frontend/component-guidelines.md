# Component Guidelines

> How components are built in this project.

---

## Overview

<!--
Document your project's component conventions here.

Questions to answer:
- What component patterns do you use?
- How are props defined?
- How do you handle composition?
- What accessibility standards apply?
-->

(To be filled by the team)

---

## Component Structure

<!-- Standard structure of a component file -->

(To be filled by the team)

---

## Props Conventions

<!-- How props should be defined and typed -->

(To be filled by the team)

---

## Styling Patterns

<!-- How styles are applied (CSS modules, styled-components, Tailwind, etc.) -->

(To be filled by the team)

---

## Accessibility

<!-- A11y requirements and patterns -->

(To be filled by the team)

---

## Common Mistakes

<!-- Component-related mistakes your team has made -->

---

## Branding / App Icons

**品牌标识**：Trove 用开放宝箱 + 字母 T 轮廓 + 中央菱形宝石镂空，应用图标源图带品牌蓝渐变（`#3b82f6` → `#2563eb` → `#1d4ed8`）；应用内组件为 `currentColor` 单色。
- 应用内 Logo 一律使用 `src/components/BrandLogo.tsx`（内联 SVG，`fill="currentColor"`），
  通过 `text-accent` 继承主题色（亮 `#2563eb` / 暗 `#5b8def`），明暗主题自动适配。
- 例：侧边栏 `<BrandLogo className="h-5 w-5 text-accent" />`（`variant="mono"` 默认），About / Onboarding
  `<BrandLogo variant="brand" className="h-16 w-16" />`（品牌蓝渐变，不依赖主题色）。

**再生成应用图标**（改 Logo 后必须执行）：

```bash
# 源图为 1024×1024 方形透明 SVG，位于 design/logo.svg
pnpm tauri icon design/logo.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios   # 项目无移动端，删除 CLI 顺带产物

# macOS 菜单栏托盘模板图标（纯黑剪影，系统渲染为白色）
npx @resvg/resvg-js-cli design/logo-tray.svg src-tauri/icons/tray-icon.png
cp src-tauri/icons/tray-icon.png src-tauri/icons/tray-icon@2x.png
sips -z 64 64 src-tauri/icons/tray-icon@2x.png
```

- **macOS 托盘**：`design/logo-tray.svg` → `tray-icon.png`；`setup_tray` 加载该图并设 `icon_as_template(true)`，菜单栏显示白色模板图标。
- **Windows 托盘**：仍用彩色 `default_window_icon()`。
- **macOS 原生 About 面板**（应用菜单 → 关于 Trove）：须在 `menu_bar.rs` 的 `AboutMetadata.icon` 显式传入
  `app.default_window_icon().cloned()`；系统不会自动用 bundle 图标更新该对话框。
- favicon：`public/logo.svg`，`index.html` 引用 `/logo.svg`。
- **Forbidden**：不要直接改 `src-tauri/icons/` 里的 PNG 产物，一律改 `design/logo.svg` 源图再生成。

**Related**: `design/logo.svg`、`src/components/BrandLogo.tsx`、`src-tauri/icons/`。

---

## Destructive/Confirm Actions

**Gotcha**: Tauri 的 webview（macOS WKWebView / Windows WebView2）不实现
`window.confirm()` 和 `window.alert()`——静默失败并返回假值。任何依赖它们的确认
或提示逻辑在真实应用里永远不执行（按钮"点了没反应"）。

**Convention**: 破坏性/确认类操作一律使用应用内两步内联确认，复用
`src/design-system/patterns/ConfirmButton.tsx`：

```tsx
<ConfirmButton
  confirmLabel="确认删除？"
  onConfirm={() => deleteMutation.mutate()}
  resetKey={task.id} // 切换选中项时自动还原
>
  删除
</ConfirmButton>
```

行为：首次点击进入"确认…？" danger 态，二次点击执行；3 秒未确认自动还原；
`resetKey` 变化时还原。成功/失败提示用内联 state（如 `notice`/`error`）渲染，
不要用 `window.alert()`。

**Forbidden**:

```tsx
// ❌ 在 Tauri webview 中永不生效
if (confirm("确认删除？")) doDelete();

// ✅ 两步内联确认
<ConfirmButton confirmLabel="确认删除？" onConfirm={doDelete}>删除</ConfirmButton>
```

**Related**: `src/design-system/patterns/ConfirmButton.tsx`、`TaskDetailPanel.tsx`、
`MemoryPage.tsx`、`ClipboardPage.tsx`、`SettingsPage.tsx`、`AttachmentsSection.tsx`。
