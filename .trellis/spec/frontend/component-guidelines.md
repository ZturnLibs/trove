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

**品牌标识**：Trove 开放宝箱 + T 轮廓 + 菱形宝石。

- **应用图标源图**（`design/logo.svg` → Dock / 窗口 / favicon）：**满铺白色方形底板**（四角不透明）；圆角由 macOS / Windows 系统在 Dock / 任务栏自动裁切，源图勿自行 `rx` 圆角（否则 Dock 四角透明、看起来无背景）。
- **应用内 UI**（`BrandLogo` 默认 `variant="brand"`）：白底圆角底板（半径 = 边长 × 22.37%），模拟系统图标外观。几何常量见 `src/components/brand-logo-assets.ts`。

- 应用内 Logo 一律使用 `src/components/BrandLogo.tsx`；默认 `variant="brand"`（白底 + 渐变）。
- `variant="mono"` 仅用于需透明底 + `currentColor` 的极少数场景。
- 例：`<BrandLogo className="h-5 w-5" />`（侧边栏 / About / Onboarding 均同）。

**再生成全部图标资产**（改 Logo 后必须执行）：

```bash
pnpm icons
```

或分步：

```bash
# 应用图标（Dock / 窗口 / About 系统面板）
pnpm tauri icon design/logo.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios

# macOS 菜单栏托盘模板（纯黑剪影，无白底）
npx @resvg/resvg-js-cli design/logo-tray.svg src-tauri/icons/tray-icon.png
cp src-tauri/icons/tray-icon.png src-tauri/icons/tray-icon@2x.png
sips -z 64 64 src-tauri/icons/tray-icon@2x.png

# favicon 与源图保持一致
cp design/logo.svg public/logo.svg
```

- **macOS 托盘**：`design/logo-tray.svg`（与主标记同 scale，透明底）；`icon_as_template(true)`。
- **Windows 托盘**：彩色 `default_window_icon()`（来自 `design/logo.svg` 生成集）。
- **macOS 原生 About 面板**（应用菜单 → 关于 Trove）：须在 `menu_bar.rs` 的 `AboutMetadata.icon` 显式传入
  `app.default_window_icon().cloned()`；系统不会自动用 bundle 图标更新该对话框。
- favicon：`public/logo.svg`，`index.html` 引用 `/logo.svg`。
- **Forbidden**：不要直接改 `src-tauri/icons/` 里的 PNG 产物，一律改 `design/logo.svg` 源图再生成。

**Related**: `design/logo.svg`、`design/logo-tray.svg`、`src/components/brand-logo-assets.ts`、`src/components/BrandLogo.tsx`、`src-tauri/icons/`。

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
